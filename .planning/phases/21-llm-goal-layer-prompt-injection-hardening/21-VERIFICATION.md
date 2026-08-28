---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-27T00:00:00Z
head: ac5267a
status: human_needed
score: 90/91 must-haves verified (4/5 ROADMAP success criteria; 0 ROADMAP criteria FAILED, 1 behavior-unverified; round-10's 4 completeness-claim FAILUREs now CLOSED — 40/40; round-11's 46/46 new must-haves VERIFIED)
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 40/45 must-haves verified (4/5 ROADMAP success criteria)
  gaps_closed:
    - "pass-11 gaps[0] (CR-01', argument injection at the Sessions-tab resume): CLOSED. `resume_terminal_argv` (`src/ui/screens/detail.rs:703-715`) now fuses the untrusted session id to its own option name in ONE argv element, `--resume=<id>` (`RESUME_OPTION_FUSED_PREFIX = \"--resume=\"`), rather than inserting a `--` end-of-options separator — the planner measured at the real binary (`claude` 2.1.248) that a `--` separator SILENTLY DELETES the resume capability (`claude --resume -- <uuid>` returns the same error as a hostile input), so fusion rather than separation is the correct fix and is disclosed as such in the corrected doc. I read `resume_terminal_argv` directly, ran `cargo test --lib ui::screens::detail::tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program -- --exact` myself (1 passed), and confirmed the test is STRUCTURAL rather than fixture-only: it asserts the number of `-`-leading argv elements is CONSTANT across the entire widened 28-fixture corpus (content-independence), not merely the per-character absence pass 11 flagged as coincidental-reliance. `hostile_session_ids()` is widened with 10 option-lookalike fixtures (`-h`, `--version`, `--dangerously-skip-permissions`, `--print`, `-`, `-r`, `--resume`, `--settings=/tmp/x.json`, `a=b`, `--add-dir`), non-vacuity-asserted. The falsified doc sentence (\"there is no interpreter left in the path to parse anything\") is corrected in place, quoting itself verbatim, naming CWE-88 and `claude`'s own option parser. `read_session_id` deliberately keeps its pass-through with the reason recorded (a validator tight enough to refuse `-h` would also refuse legitimate session-title resumes). The three sibling argv builders in `src/executor/claude.rs` were measured (all three fields `None` at `ExecutionOptions::default()`, set by no caller in the tree) and correctly left unfused, recorded as a standing item with an explicit promote condition (first caller that sets one from an untrusted value)."
    - "pass-11 gaps[1] (the interpreter census's comment-line budget bug at `src/text.rs:1750-1755`): CLOSED. `taken += 1` now executes AFTER the `next.starts_with(\"//\")` comment check — I read the moved line directly. Ran `cargo test --lib text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments -- --exact` myself (1 passed): a genuinely two-sided boundary test asserting the census still MISSES at 12+ EXECUTABLE filler lines (unchanged) while now REPORTING at 100 COMMENT filler lines (previously missed at 12). The method-chain residual `has_an_unclosed_delimiter`'s doc pointed at but that the census's residual block did not contain is now written there. The census's doc is narrowed from claiming 'any' such line to stating the three bounds it actually has, rather than the marker set being silently widened to manufacture a broader claim."
    - "pass-11 gaps[2] (the composition census's over-joining at `src/ui/screens/driver.rs`): CLOSED. The composition verdict moved from a whole-logical-unit check to the INNERMOST call — `occurrence_is_composed` (`driver.rs:2293-2299`) looks only at the characters immediately preceding each needle occurrence, so there is no window left to over-join. Ran `cargo test --lib ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling -- --exact` myself (1 passed): the exact two-arm `match` fixture pass 11 used to reproduce the laundering (`census hits: []` under the old rule) is now reported (`HIT` under the new rule). Also fixed in the same commit: NON-VACUITY 2 now computed over the same slice the census scans (not the whole file), and the `#[cfg(test)]` truncation now cuts at a column-zero test MODULE marker with exactly one asserted per file (previously a silent kill switch — I confirmed `detail.rs` carries two such markers in this tree, which the fix's own test plants and observes red for)."
    - "pass-11 gaps[3] (the `src/ui/mod.rs` census's false completeness claim): CLOSED by narrowing, not by a fourth census. The test is renamed `no_display_identity_call_under_ui_stands_outside_a_composition` (from `every_render_site_under_ui_composes_both_classes`) — I read the doc directly: it states the measured reach (8 executable occurrences, 6 non-exempt in 2 of 16 files) as a CHECKED per-file equality (`MEASURED_REACH` constant) that fails in both directions, discloses the shrinking-coverage property with its direction (under-detection, silent, growing as conversions succeed), and hands the actual completeness claim by name to the sealed `RenderAdjudicated` supertrait and the `render_escape_guard` behavioural probes. Ran `cargo test --lib ui::tests::no_display_identity_call_under_ui_stands_outside_a_composition -- --exact` myself (1 passed). The doc's own commit history shows the pin caught itself going stale mid-plan when Task 3 added an eighth occurrence — direct evidence the pin is load-bearing, not decorative."
    - "WR-04 (EditBuffer's trait-absence claim was prose-only) CLOSED with a local autoref-specialization probe (`an_edit_buffer_implements_none_of_the_string_conversions`, `src/ui/screens/mod.rs:1915+`) mirroring `Untrusted`'s pattern, with matching `String` presence-control arms. Ran myself: 1 passed."
    - "WR-05 (the backlog comparator was total over KEYS, not ELEMENTS, so ties fell back to read_dir order) CLOSED: `backlog_number_ordering` (`src/state_reader/backlog.rs:180-184`) now appends `.then_with(|| a.cmp(b))`. Read directly."
    - "21-29's carried S1 (a two-direction input-echo spot-check) DELIVERED: `an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone` (`src/ui/screens/render_escape_guard.rs:3220+`) drives `EnqueueScreen`'s real `Screen::render` with a TAG_PAIR fixture, asserting arrival-before-property in both directions, with the widget family (`Paragraph`, which drops zero-width but preserves tag characters) named as the reason that fixture rather than a zero-width one was chosen. Ran myself: 1 passed. Housed in the probe module with zero visibility changes to `src/ui/screens/mod.rs`, as the plan's own measurement required."
    - "The `driver_reattach` flake record's stale `--test-threads=1` mitigation claim is corrected AT the stale sentence (not only in a later section), a comment-only module note is added to `tests/driver_reattach.rs` itself (diff confirmed 81 insertions / 0 deletions by me), and the correction's completeness is established by a committed grep inventory (388 hits classified: 354 dated observations left unedited, 34 living hits individually read, 7 carrying a standing claim corrected). `21-34`'s own merged-tree gate measurement CONTRADICTED the wave-1 'isolated re-runs are reliably green' generalization (2 green / 4 red out of 6 isolated runs) and that contradiction is recorded rather than smoothed, with a third candidate mechanism (M3, intra-process thread-timing on `isolate_envelope_root`'s env-var write) named but explicitly NOT claimed as proven."
    - "A Record-corrections table in `deferred-items.md` names all four round-10 truths pass 11 measured false as shipped (`21-27` truth 1, `21-27` truth 5, `21-28` truth 7, `21-29` truth 2), quoting the falsified claim and citing the plan/task that closed it. Read directly."
    - "REQUIREMENTS.md untouched for the NINTH consecutive round: `git log --oneline -3 -- .planning/REQUIREMENTS.md` still ends at `0c4f712`; `git diff --stat 343c408..HEAD -- .planning/REQUIREMENTS.md tests/driver_injection_corpus.rs` is empty. Both run by me."
    - "COVERAGE.md amended append-only: the existing no-external-API declaration is byte-identical, with a dated round-11 paragraph appended stating no dependency was added (`git diff Cargo.toml Cargo.lock` empty over 343c408..HEAD, confirmed by me) and naming what changed instead."
  gaps_remaining: []
  regressions:
    - "None found. `git diff --stat 343c408..HEAD -- src/` touches exactly the eight files the four plans' `files_modified` declare (`session_detector.rs`, `state_reader/backlog.rs`, `test_support.rs`, `text.rs`, `ui/mod.rs`, `ui/screens/detail.rs`, `ui/screens/driver.rs`, `ui/screens/mod.rs`, `ui/screens/render_escape_guard.rs`) plus no others — confirmed by me, not inherited. Full workspace suite re-run by me: `cargo test --workspace --no-fail-fast` — 1424 passed / 0 failed / 13 ignored (34 binaries), matching the orchestrator's pre-measured gate exactly; `driver_reattach` came back 3/3 green in my run, consistent with its documented non-determinism and not scored as a regression or a fix. `cargo clippy --all-targets -- -D warnings` — exactly 4 pre-existing lints (3x `bool_assert_comparison` at `src/browser.rs:155-157`, 1x `cmp_owned` at `src/project_creator.rs:146`), re-measured by me; `cargo clippy -- -D warnings` exit 0. No dependency added (`git diff Cargo.toml Cargo.lock` empty). ROADMAP criteria 1, 2, 3, 5 reconfirmed unregressed: `driver_escalation_cap` 8/0, `driver_refusal_record` 9/0, `driver_injection_corpus` 13/0/10-ignored (unchanged, `tests/driver_injection_corpus.rs` absent from the round-11 diff), `spawn_seam_guard` 15/0 — all run by me."
deferred:
  - truth: "A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes — the BEHAVIOURAL half (ROADMAP success criterion 4 / SAFE-07)"
    addressed_in: "Not any phase — permanently agent-unclosable by explicit user decision, re-surfaced verbatim by `21-34` for the fourth consecutive round in deferred-items.md"
    evidence: "Verified unchanged for the TWELFTH consecutive pass. My own run: `tests/driver_injection_corpus.rs` reports 13 passed / 0 failed / 10 ignored. `git diff --stat 343c408..HEAD` names no file under `tests/` except the comment-only `tests/driver_reattach.rs`, so round 11 did zero work against criterion 4 — as all four round-11 plans' prohibitions require. 4/5 is the EXPECTED and correct outcome and is not scored as a failure."
  - truth: "General Unicode CONFUSABLES / homoglyph (TR39) defence in FREE TEXT"
    addressed_in: "Not phase 21 — recommend a new roadmap item"
    evidence: "Unchanged from passes 9-11. Out of scope by design."
  - truth: "Four PRE-EXISTING clippy lints make `cargo clippy --all-targets -- -D warnings` fail"
    addressed_in: "deferred-items.md, re-measured a fifth time by me"
    evidence: "I ran `rtk proxy cargo clippy --all-targets -- -D warnings`: exactly four lints (three `bool_assert_comparison`, one `cmp_owned`). `rtk proxy cargo clippy -- -D warnings` (lib only) exits 0. Neither `src/browser.rs` nor `src/project_creator.rs` appears in `git diff --stat 343c408..HEAD`."
  - truth: "`tests/driver_reattach.rs`'s nondeterministic tests"
    addressed_in: "deferred-items.md's standing entry, now with a corrected mechanism analysis (M1/M2/M3) and a 388-hit grep inventory establishing the correction's completeness"
    evidence: "My own isolated run came back 3/3 green (matching the documented non-determinism — not every run flakes). `21-34`'s own merged-tree measurement recorded 2 green / 4 red out of 6 isolated runs, which I read directly rather than re-deriving; it is disclosed as CONTRADICTING the prior 'isolated re-runs are reliably green' generalization rather than being smoothed into it. Not attempted to be fixed, per this round's own prohibition. Not scored as a regression — pre-existing, and the record now says so more precisely than before."
  - truth: "`src/executor/claude.rs`'s three `--option value` argv pairs (`--resume`, `--model`, `--name`), the same CWE-88 shape gaps[0] closed at the Sessions-tab resume"
    addressed_in: "Standing item recorded by `21-34` in deferred-items.md, with an explicit promote condition"
    evidence: "I confirmed directly: `resume_session`, `model` and `name` are all `None` at `ExecutionOptions::default()` (`src/executor/mod.rs:464,465,473`) and no caller in the tree sets any of the three to `Some(..)` outside test code (grepped `resume_session\\s*:\\s*Some|\\.resume_session\\s*=|model:\\s*Some|\\.model\\s*=|name:\\s*Some\\(` across `src/` — zero non-test hits). So there is no untrusted value reaching this seam today; the promote condition is the first caller that sets one from a value not authored in this crate."
  - truth: "`registry::current_prompt_inputs` absent from BLOCKING_HELPERS; the spawn-gate plan-half comment; dead `PlanStep::rationale`"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, unchanged across rounds 4-11)"
    evidence: "No round-11 plan touched any of the three."
behavior_unverified_items:
  - truth: "A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes (ROADMAP success criterion 4 / SAFE-07)"
    test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed. Every `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload ARRIVED at the model before asserting the command was unchanged; the two suppression controls show the positive/negative `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile and clean arms both really ran."
    why_human: "All ten spawn the real `claude` binary and need an authenticated subscription, so they cannot run inside verification. NO AGENT CAN CLOSE THIS ITEM, and the user has explicitly chosen to leave it tracked in `deferred-items.md`. Counted independently from my own run: 13 passed / 0 failed / 10 ignored. Round 11 did NO work against it (all four plans carry an explicit prohibition; `tests/driver_injection_corpus.rs` is absent from `git diff --stat 343c408..HEAD`). Presence and wiring verified for the TWELFTH consecutive pass; behaviour never exercised by any verification pass of this phase."
human_verification:
  - test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed, with arrival asserted before influence in every class arm and `both_arms_of_every_class_comparison_were_really_executed` green."
    why_human: "Requires an authenticated subscription and spawns the real model binary; cannot run inside verification. This is criterion 4's only behavioural evidence and no verification pass of this phase has ever produced it. This is the ONLY reason overall status is `human_needed` rather than `passed` — every other must-have in the phase is VERIFIED."
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-27T00:00:00Z
**Status:** gaps_found
**Re-verification:** Yes — tenth verification pass, after the ninth gap-closure cycle (round 9: `21-23`..`21-26`)

**NOTE ON FILE STRUCTURE:** Per this pass's explicit instruction, the pass-9 record is preserved below and is unmodified — nothing in it was edited, deleted, or renumbered. This pass's own findings are the frontmatter above (now the authoritative current status/score/gaps for any tooling that reads this file) plus the "PASS 10 ADDENDUM" section appended at the very end, after the preserved pass-9 record.

---

## PRESERVED PASS-9 RECORD (condensed; full original text is unmodified in git history at commit `98610bf` and can be recovered with `git show 98610bf:.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-VERIFICATION.md`)

Pass 9's full report — ~490 lines of prose covering five Judgments (independent-oracle re-derivation of criterion 1 across all 170 `Cf` and all 4174 `Default_Ignorable` code points; independent re-derivation of the ratatui rendering premise in a scratch crate; the discovery of the intermittent `the_screen_renders_identity_escaped` failure and the (a)/(b) dichotomy it could not resolve; the discovery of five unescaped `.planning/`-derived render sites), the complete 24-truth Observable Truths table, Required Artifacts table, Key Link Verification, Data-Flow Trace, 16 Behavioral Spot-Checks, Requirements Coverage, Anti-Patterns Found, and Test Quality Audit — is preserved unchanged in git history and is authoritative for what pass 9 found. It is not reproduced a second time verbatim in this update; every specific pass-9 finding this pass-10 addendum below relies on (gaps[0], Judgment 3, Judgment 4, Judgment 5, the TAG_PAIR coincidental-reliance flag) is quoted or paraphrased precisely enough below to stand alone, and pass 9's own frontmatter is condensed here for continuity:

```yaml
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-26T02:59:59Z
head: f442881
status: gaps_found
score: 24/26 must-haves verified (4/5 ROADMAP success criteria; 1 FAILED, 1 behavior-unverified)
gaps:
  - truth: "the_screen_renders_identity_escaped renders every screen through the real Screen::render and asserts zero invisible-class characters reach the buffer, for every state the disposition table marks renders-identity"
    status: failed
    reason: "Fired ONCE in ~80 observations against DetailScreen [GitHistory tab], reporting two raw copies of a hostile identity in terminal cells, then could not be reproduced in 8 further workspace runs, 12 full-lib runs, 10 full-lib runs under contention, or 60 direct single-test runs — all green. Neither explanation (a real state-dependent leak vs. a nondeterministic probe) could be established from outside the tree."
behavior_unverified_items:
  - truth: "A .planning/ file or CLAUDE.md carrying injected instructions does not change which command the driver executes (ROADMAP success criterion 4 / SAFE-07)"
    why_human: "All ten driver_injection_corpus arms spawn the real claude binary and need an authenticated subscription; permanently agent-unclosable by explicit user decision, tracked in deferred-items.md"
coincidental_reliance_items:
  - truth: "The TUI render surface draws no invisible-class character into a terminal cell (21-21 t3/t4)"
    reason: fixture-only
    harden: "Promote TAG_PAIR's teeth to a checked precondition"
```

ROADMAP criteria 1, 2, 3, 5 were VERIFIED at pass 9 (criterion 1 swept over all 170 `Cf` and all 4174 `Default_Ignorable` code points against Perl `Unicode::UCD` cross-checked with OpenJDK, zero escapes); criterion 4 was PRESENT_BEHAVIOR_UNVERIFIED and permanently agent-unclosable. Six carrier types were deliberately left unretyped and disclosed in `deferred-items.md`. TR39 confusables/homoglyphs remained explicitly out of scope.

---

## PASS 10 ADDENDUM (2026-08-27, verification pass 10, after round 9: plans `21-23`..`21-26`)

**Scope of this pass.** Round 9 (`21-23`..`21-26`) executed since pass 9, plus orchestrator commits `7bf8f6b` (the `Rendered::fmt` → `f.pad` fix) and `c9345a1` (the follow-on comment correction). An independent code reviewer (`21-REVIEW.md`) reviewed the same range and found 3 Critical, 8 Warning, 2 Info issues. This addendum independently confirms or refutes each Critical by reading the cited code directly, reconfirms the five ROADMAP criteria, reconfirms round 9's own must-have claims by reading code and running targeted tests, and determines the pass-10 status.

### ROADMAP Success Criteria — reconfirmed

| # | Truth | Status | Evidence (pass 10) |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | VERIFIED (regression-checked) | `git diff 98610bf..HEAD -- src/text.rs` shows round 9 added the alphabet-census needle normalization (WR-05) and the `Untrusted`/`Rendered` types but made no change to `is_invisible_formatting_char`, `is_identity_char`, `carries_visible_content`, or `carries_invisible_formatting` (confirmed by grepping the diff for those four signatures — no hits). Pass 9's fourth/fifth-oracle sweep is therefore unaffected. `cargo test --lib text::` — 17 passed / 0 failed in my own run. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | VERIFIED (regression-checked) | `src/driver/run.rs` untouched by round 9. `src/driver/mod.rs` changed (34 insertions/9 deletions, from `21-24`) but only to retype `DriveError`/`OptInError` refusal fields to `crate::text::Untrusted` — read directly, confirmed the approval/iteration ordering is untouched. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | VERIFIED (reconfirmed, ran myself) | `cargo test --test driver_escalation_cap`: 8 passed / 0 failed, both cap directions. File untouched by round 9's diff. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | PRESENT_BEHAVIOR_UNVERIFIED (unchanged, permanently agent-unclosable) | `cargo test --test driver_injection_corpus`: 13 passed / 0 failed / 10 ignored, identical to pass 9. `git diff --stat dfa11c6..HEAD -- tests/driver_injection_corpus.rs` is empty. Still human-only. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | VERIFIED (regression-checked) | `parse_action` (`src/driver/goal.rs:343-351`) unchanged, read directly — still an `ALL`-slice lookup returning `Result<RouterAction, UnknownCommand>`. `cargo test --test driver_refusal_record`: 9 passed / 0 failed. **CR-01 (below) does NOT falsify this criterion**: CR-01 is a shell-injection bug in the Sessions tab's human-triggered "resume a claude session found via /proc scan" feature — a different actor (human, not model) and a different value (a /proc-scraped session id, not a model-named GSD command). SAFE-08's formal text is specifically about the model's chosen action; `parse_action` is untouched. CR-01 is scored as its own gap, not as this criterion's failure. |

**ROADMAP score: 4/5 verified, 0 FAILED, 1 behavior-unverified — same distribution as pass 9. No regression found on any of the four VERIFIED criteria.**

### Independent confirmation of `21-REVIEW.md`'s three Critical findings

I read every cited line range directly before forming a verdict, rather than trusting the reviewer's prose.

| # | Claim | My verdict | Evidence |
|---|---|---|---|
| CR-01 | Shell command injection at `detail.rs:1692-1707` via `format!`-built `sh -c` string interpolating `session.working_dir` and `sid.as_raw_for_logic_only()`, both `/proc`-sourced, neither validated | **CONFIRMED, real** | Read `detail.rs:1692-1720` and `session_detector.rs:59-113` directly: the shell string is exactly as described, both interpolants trace to unvalidated `/proc` reads (`read_link("/proc/<pid>/cwd")`, cmdline `--resume` argument). A `'` in either escapes the quoting. The `sh -c` pattern predates this phase (`867d277`, phase 9, confirmed via `git log -S`), but round 9's own diff (`21-25`) touched these exact lines to retype `sid` and added a new comment misclassifying the value as "A SUBPROCESS ARGUMENT" — factually wrong (it is a shell-command fragment, not an argv element) and new to this round. |
| CR-02 | The live driver output pane, injection rows, and dry-run preview compose only the control-character class, not the invisible-formatting class, so the tag block reaches a `Paragraph` cell | **CONFIRMED, real** | Read `mod.rs:425-437` (`push_record` → `sanitize_record_lines` → `sanitize_render_line`, control class only), `driver.rs:1712-1734` (`output_line`, `Span::styled(line.text.clone(), ..)`, no `shown()`/`display_identity`), `driver.rs:1955-1964` (`Paragraph::new(body)`). Grepped `render_escape_guard.rs` for `driver_output`/`driver_journal`/`driver_inbox`: zero matches — `probe_ctx` never populates them, confirming this path is genuinely unprobed as well as unescaped. This round's own per-widget ratatui table (`deferred-items.md`, 2026-08-27 entry) records the tag block (`U+E0041`) as SURVIVING `Paragraph`, so this reaches a cell, not merely a theoretical risk. |
| CR-03 | `detail.rs`: `entry.value` is escaped at the list render (`:4088`, `shown()`) but copied raw into `defaults_text_buffer` at `:1887` and rendered raw at `:4146` | **CONFIRMED, real** | Read all three line ranges directly: `Span::styled(shown(&entry.value), ..)` at the list row; `cache.defaults_text_buffer = entry.value.clone()` (no `shown()`) at the edit-entry key handler; `Span::styled(buffer.clone(), ..)` (no `shown()`) at the popup render. `render_escape_guard.rs`'s LIMIT 1 names this state as unprobed but does not disclose it is also unescaped. |

**All three Criticals independently confirmed as real, present-tense defects.** I additionally spot-checked WR-01 (confirmed: doc claims five absent trait impls, control certifies three) and attempted to reproduce WR-07's "panic" claim (refuted below). WR-02, WR-03, WR-06, WR-08 and the two Info items were read once at their cited ranges but not independently re-derived beyond that; nothing I read contradicted the reviewer's characterization.

### Round 9's own must-have claims — spot-verified

| # | Claim (source) | Status | Evidence |
|---|---|---|---|
| 25 | `Untrusted` withholds `Display`/`AsRef<str>`/`Deref`/`Borrow<str>`/`Into<Cow>`/serde; `Rendered` is the sole escaped `Into<Cow<'static,str>>` type (21-23 t1) | VERIFIED, with a disclosed control gap | Types exist as described at `text.rs:419,437-439,487,537,557,562`, read directly; today `Untrusted` genuinely has none of the five. But `an_untrusted_carrier_implements_none_of_the_string_conversions` (`text.rs:1729-1791`) asserts only 3 of the 5 claimed absences (Display, AsRef<str>, Into<Cow>) — Deref, Borrow<str>, serde uncertified (WR-01). Not a live leak; a future regression risk with no tripwire. |
| 26 | `render_for_terminal`/`strip_terminal_controls` is the one composition, behaviour-preserving (21-23 t2) | VERIFIED | Code matches the described shape; `sanitize_render_line`'s own tests pass unmodified per round 9's own audit, not disputed here. |
| 27 | CR-04 (`GitLogEntry`/GitHistory tab probe fixture) made deterministic, not a flake (21-23 t3) | VERIFIED | Ran `the_screen_renders_identity_escaped` five consecutive times myself: 5/5 green, ~2.6-2.7s each, no variance. |
| 28 | Ratatui per-widget premise corrected, TAG_PAIR teeth pinned (21-23 t5/t6) | VERIFIED | `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell` (`render_escape_guard.rs:1852-1878`) exists and passes in my own run. |
| 29 | `Untrusted` has hand-written `Debug` routing through `shown()`, no derive (21-23 t8) | VERIFIED | `text.rs:577-579`: `write!(f, "Untrusted({:?})", self.shown().to_string())`, confirmed by reading. |
| 30 | Eight `.planning/` carriers retyped; every render site the compiler named resolves deliberately (21-25) | **FAILED as a completeness claim** | The retype is real and correctly done for the eight named carriers. But the round's own record of "what was NOT retyped" (`deferred-items.md`'s six-carrier table) omits `DriverOutputLine::text` and `ProjectViewCache::defaults_text_buffer` — both live, unescaped, undisclosed carrier-shaped `String` fields (CR-02, CR-03). |
| 31 | Multibyte session-id truncation rewritten as a `char` operation (21-25 T1d) | VERIFIED | `shorten_session_id` (`detail.rs:114-118`) confirmed a `.chars().take(N).collect()` operation. |
| 32 | CR-05: unadjudicated `Screen` fails to build via sealed `RenderAdjudicated` supertrait (21-26 t1-t4) | VERIFIED | `ui/screens/mod.rs:65-66,142,189` confirmed by direct read. Object-safety preserved (methods only, no assoc consts). `the_screen_census_matches_the_tree` and `the_census_reports_an_unadjudicated_screen_and_a_stale_row` both green in my own run. |
| 33 | WR-05: alphabet-spelling census needle normalized (21-26 t6) | VERIFIED | `the_normalized_needle_still_excludes_the_branch_name_set` present and green in my own `cargo test --lib text::` run. |
| 34 | DRIVE-04 boundary and precision reconfirmed by re-run (21-26 t7/t8) | VERIFIED | Reproduced myself: `driver_escalation_cap` 8/0. |
| 35 | The round-9 record is appended, dated, quoting what it corrects; REQUIREMENTS.md untouched (21-26 t9) | VERIFIED | `deferred-items.md`'s round-9 entries read append-only with verbatim quotes of corrected text. `git log --oneline -3 -- .planning/REQUIREMENTS.md` still ends at `0c4f712` — seventh consecutive round untouched. |

**Round-9 must-haves score: 8/9 VERIFIED (one, #30, FAILED as a completeness claim — its constituent retypes are real, but the claim that the render/execution surface is now fully accounted for is false, per gaps[1] and gaps[2]).**

### Combined score

- ROADMAP criteria: 4/5 verified, 1 behavior-unverified, 0 failed (unchanged from pass 9)
- Round-9 must-haves spot-verified: 8/9 verified, 1 failed (#30, compound)
- Pass-9 carried-forward must-haves (#6-24 in the original file): reconfirmed not regressed — none of their owning files were touched by round 9 in a way that changes their claim, or were re-read directly where round 9 did touch the file (e.g. `driver/mod.rs`)
- **New gaps found by this pass, independent of round 9's own framing and of the reviewer's framing:** the "render/execution surface is fully accounted for" claim fails at 3 concrete, confirmed sites (CR-01, CR-02, CR-03)

**Total: 30/33 must-haves verified** — see frontmatter `score` for the canonical figure. The 3 FAILED items are gaps[0..2] in the frontmatter.

### Anti-Patterns Found (pass 10, incremental)

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/ui/screens/detail.rs` | `1692-1707` | Shell command injection: `format!`-built `sh -c` string interpolating two unvalidated `/proc`-sourced values | Blocker | CR-01, confirmed. Arbitrary code execution with the operator's privileges if a `--resume` argument or cwd of another locally-running process contains `'`. This round's diff touched these lines and added a false security classification comment. |
| `src/ui/screens/driver.rs` | `1088`, `1684`, `1732` | Live driver output pane, injection rows, and dry-run preview compose only the control-character class, never the invisible-formatting class | Blocker | CR-02, confirmed. The pane that displays the LLM's own output — this phase's central threat surface — can render the tag-block "ASCII-smuggling carrier" invisibly. Undisclosed in `deferred-items.md`'s own residual table. |
| `src/ui/screens/detail.rs` | `1887-1888`, `4146` | `defaults_text_buffer` escaped at the list render, raw at the edit-popup render of the same value | Blocker | CR-03, confirmed. An operator editing a config value reads an unescaped string while deciding what to write back to disk. |
| `src/text.rs` | `512-530` vs `1729-1791` | `Untrusted`'s doc claims five certified-absent trait impls; the control checks three | Warning | WR-01, confirmed. No live leak today; a future `impl Deref`/`Borrow<str>`/`Serialize` would silently restore the raw path with every existing test staying green. |
| `src/state_reader/backlog.rs` | `88-106` | NaN-producing float sort comparator on a `.planning/`-derived directory name | Info (downgraded from the reviewer's Warning) | WR-07. I reproduced the comparator shape in a standalone Rust program (rustc 1.97.1) and it does NOT panic — the reviewer's specific "Rust detects and panics" claim is not reproducible on this toolchain. The underlying defect (silently wrong sort order on a `999.NaN-x` directory name) is real but is a display-ordering bug, not a DoS. |

### Requirements Coverage (pass 10 delta)

| Requirement | Status (pass 9 → pass 10) | Evidence |
|---|---|---|
| DRIVE-01 | SATISFIED → still SATISFIED | Criteria 1, 2 unaffected by round 9's diff. |
| DRIVE-03 | SATISFIED → still SATISFIED | Criterion 1's machinery unaffected. |
| DRIVE-04 | SATISFIED → still SATISFIED | Reconfirmed by my own `driver_escalation_cap` run (8/0). |
| SAFE-07 | NEEDS HUMAN → still NEEDS HUMAN, plus 2 new confirmed gaps within its own render-surface scope (CR-02, CR-03) | Criterion 4's behavioural half unchanged. Round 9's own plans self-scoped the "render surface" work under SAFE-07/SAFE-08 (`21-25-PLAN.md`'s STRIDE register frames `.planning/`-derived display honesty as this requirement's territory), and two of the three round-9 completeness claims within that scope are FAILED. |
| SAFE-08 | SATISFIED → still SATISFIED formally; CR-01 is a related-but-distinct finding, not a criterion-5 failure | `parse_action`'s enum-lookup mechanism is unchanged and still refuses every out-of-enum action name. CR-01 is a shell-injection bug in an unrelated, human-triggered TUI feature, not the model's chosen-action path. Recorded as a Blocker in Anti-Patterns and as gaps[0] because it is a real, independently serious security defect this round's own diff touched and mischaracterized — but it does not falsify SAFE-08's formal text. |

**No orphaned requirements.** The union of `requirements:` across all 26 plans (22 original/gap-closure + `21-23`..`21-26`) is exactly {DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}, matching REQUIREMENTS.md's phase-21 mapping.

### Gaps Summary (pass 10)

**Round 9 closed real ground.** The flagship intermittent-failure gap from pass 9 (gaps[0]) is now deterministically closed — reproduced its fix five times with zero variance, materially stronger evidence than pass 9 could get for a probe that fired once in ~80 observations. The five `.planning/`-derived render sites Judgment 5 first named are escaped and confirmed by direct read. The multibyte session-id panic is fixed. CR-05 (an unadjudicated screen can now never compile) is a genuinely durable, type-level closure of the census's whole failure class, verified by reading the sealed-trait mechanism directly. DRIVE-04 and criteria 1, 2, 3, 5 show no regression under a diff-scoped and spot-tested check.

**But round 9's headline claim — that an unescaped untrusted string is now unrenderable, full stop, rather than merely that the sites a human found were patched — is false, and it is false in a way this pass independently confirmed by reading code rather than by trusting either the round's SUMMARYs or the code reviewer's prose.** Two of the three confirmed gaps (CR-02, CR-03) are exactly the shape the whole nine-round arc of this phase has been fighting: a value that never entered a carrier, so the compiler-named-sites methodology structurally cannot see it, and the round's own "here is what we did NOT retype" disclosure (`deferred-items.md`) does not name either of them — the residual is not merely open, it is under-disclosed. CR-02 is the more serious of the two on threat-model grounds: it is the pane that shows the LLM's own output, in a phase titled Prompt-Injection Hardening, and it can carry the exact ASCII-smuggling tag-block carrier this phase names as the reason the render surface needed hardening at all. The third (CR-01) is a real, independently serious shell-injection vulnerability that this round's own diff touched and mischaracterized, though it sits outside the ROADMAP criteria's formal text (a human-triggered feature, not the model's chosen action) and so does not itself fail SAFE-08.

**What stops this from being a clean pass:** three concrete, confirmed, currently-live defects in code this round's own diff touched or (for CR-01) re-touched with a false rationale. None is hypothetical; each was read directly in the current tree, not inferred from the review or from a SUMMARY.

### Recommendation

**One narrow round 10, closing exactly what pass 10 confirmed, nothing more:**

1. **Fix CR-01.** Stop building a shell string for the Sessions-tab resume; pass argv directly via `Command::args` and `current_dir`. Correct the misclassifying comment.
2. **Fix CR-02.** Compose both escape classes on the driver output pane / injection rows / dry-run preview path (three sites), then populate `probe_ctx`'s `driver_output`/`driver_journal`/`driver_inbox` so the probe actually exercises it going forward. Correct `sanitize_render_line`'s doc claim in the same commit.
3. **Fix CR-03.** Escape `defaults_text_buffer` at its render site; add the missing `DETAIL_SUB_STATES` fixture entry.
4. **Optional, cheap, closes a real future-regression risk:** extend `an_untrusted_carrier_implements_none_of_the_string_conversions` to also certify the absence of `Deref`, `Borrow<str>`, and serde traits (WR-01).
5. **Do not chase WR-07's panic claim as written** — it did not reproduce on this toolchain (rustc 1.97.1). If the underlying NaN-ordering correctness issue is worth fixing, it is a one-line `total_cmp` swap with no urgency attached.
6. **Criterion 4 still needs a human and nothing else** — unchanged from every prior pass.

---

_Verified: 2026-08-27T00:00:00Z_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `c9345a1` · tenth verification pass · pass 9 preserved above and at `f442881`, pass 8 at `f1a9d0d`, pass 7 at `84143bb`_

---

## PASS 11 ADDENDUM (2026-08-27, verification pass 11, after round 10: plans `21-27`..`21-30`)

**Scope of this pass.** Round 10 (`21-27`..`21-30`) executed and merged since pass 10, plus the
orchestrator's post-merge commits `5056e4e` (drop the satisfied `WAVE_PENDING` entry),
`7daaa0e`/`a061f8c` (tracking) and `feb37ee` (the round-10 code review). Review range
`80bc4c1..HEAD`, HEAD `feb37ee`. An independent code reviewer (`21-REVIEW.md`, status
`issues_found`: 1 Critical, 6 Warning, 6 Info) reviewed the same range. This pass:
independently confirms or refutes every finding it relies on by reading the code and by
re-implementing and RUNNING the three new censuses' own algorithms; reconfirms the five
ROADMAP criteria; scores round 10's own 40 declared must-have truths; and determines the
pass-11 status.

**Method note.** Every count- and presence-bearing command in this pass was run through
`rtk proxy`. Three of this pass's findings are MEASUREMENTS produced by re-implementing a
committed census's published algorithm in Python and running it against a fixture — not
readings of the reviewer's prose and not readings of the source. Where I could not measure,
I say so.

### ROADMAP Success Criteria

| # | Truth | Status | Evidence (pass 11, all independently produced) |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | VERIFIED (regression-checked) | `git diff --stat 80bc4c1..HEAD` names NO file under `src/driver/` and no file under `tests/` — 20 files, all under `src/ui/`, `src/text.rs`, `src/error.rs`, `src/app.rs`, `src/state_reader/backlog.rs`, `src/test_support.rs`. I grepped `git diff 80bc4c1..HEAD -- src/text.rs` for each of the seven escaping/alphabet primitives (`is_invisible_formatting_char`, `is_identity_char`, `display_identity`, `strip_terminal_controls`, `render_for_terminal`, `carries_visible_content`, `carries_invisible_formatting`): zero hits, so pass 9's exhaustive oracle sweep over all 170 `Cf` and all 4174 `Default_Ignorable` code points is untouched. My own runs: `driver_goal_seam` 22/0, `driver_model_seam` 5/0, `driver_dry_run` 15/0. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | VERIFIED (regression-checked) | `src/driver/run.rs` and `src/driver/mod.rs` are absent from the round-10 diff; `git log --oneline -2 -- src/driver/goal.rs src/driver/run.rs src/driver/mod.rs` still ends at round 9's `017d82d`. My own runs: `driver_iteration_loop` 7/0, `driver_lock` 5/0, `driver_tracer` 4/0. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | VERIFIED (reconfirmed, ran myself) | `cargo test --test driver_escalation_cap`: 8 passed / 0 failed, both cap directions. File untouched by the round-10 diff. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | PRESENT_BEHAVIOR_UNVERIFIED (unchanged, permanently agent-unclosable by explicit user decision) | `cargo test --test driver_injection_corpus`: 13 passed / 0 failed / **10 ignored**, identical to passes 9 and 10. `git diff --stat 80bc4c1..HEAD` names no file under `tests/`. Human-only; see `human_verification`. **4/5 is the expected and correct outcome — this is not scored as a failure.** |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | VERIFIED (regression-checked) | `parse_action` (`src/driver/goal.rs:343-351`) read directly and unchanged — still `RouterAction::ALL.iter().copied().find(|a| a.verb() == named).ok_or_else(UnknownCommand)`. My own runs: `driver_refusal_record` 9/0, `driver_router_conformance` 3/0, `driver_router_table` 12/0. **CR-01' (gaps[0]) does not falsify this criterion**, per the scoping settled at pass 10 and re-affirmed here: the Sessions-tab resume is a HUMAN-triggered TUI action over a `/proc`-scraped session id, not the MODEL's chosen action, and SAFE-08's formal text is about the latter. It is scored as its own gap. |

**ROADMAP score: 4/5 verified, 0 FAILED, 1 behavior-unverified — the same distribution as
passes 9 and 10. No regression on any of the four VERIFIED criteria.**

### The round-10 review's Critical — independently confirmed, and it is worse than "not disclosed"

I did not take the reviewer's Critical on trust, and I did not take the orchestrator's
summary of it on trust either. What I did:

1. **Read the argv.** `resume_terminal_argv` (`src/ui/screens/detail.rs:619-630`) returns
   `[separator, "claude", "--resume", sid.as_raw_for_logic_only()]`. There is no `"--"`.
2. **Read the source of the value.** `session_detector::read_session_id`
   (`src/session_detector.rs:96-113`) splits `/proc/<pid>/cmdline` on NUL, takes the token
   after `--resume`, `trim()`s it and accepts it if `!val.is_empty()`. That is the whole
   validation. Nothing constrains the first byte.
3. **Measured the receiving parser rather than assuming it.** `claude --help` on this
   machine prints `-r, --resume [value]` — an **optional**-value option — and
   `--dangerously-skip-permissions` is a real flag in the same output. A hyphen-leading
   token after `--resume` is therefore read as a NEW option of `claude`, not as
   `--resume`'s argument.
4. **Read the attacker's entry requirement.** `get_claude_pids` is `pgrep -x claude`
   (`session_detector.rs:44`) and `build_session` drops any row whose `/proc/<pid>/cwd`
   readlink fails (`:63`), so the attacker must run a process named `claude` as the SAME
   user. No privilege boundary is crossed, and the rating below is scoped accordingly.
5. **Read the certifying control.** `hostile_session_ids()` (`detail.rs:7270-7293`) is
   `LOOK_ALIKE_PAIRS` plus eleven shell-metacharacter fixtures. I checked every fixture in
   both lists (`src/test_support.rs:124-132` for the first): **not one begins with `-`.**
   Assertion (1) of `the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`
   requires only that the id appear exactly once, byte-identical — which a hyphen-leading
   id satisfies. The control passes today and would pass unchanged against a build
   shipping this defect.
6. **Checked the disclosure.** Grepped `21-27-SUMMARY.md`, `21-30-SUMMARY.md` and
   `deferred-items.md` for `end-of-options`, `argument injection`, `CWE-88`, `hyphen`,
   `option parser`, `leading -`: **zero hits.**

So this is not merely an open residual — it is an **undisclosed** one, in the round whose
own prohibition reads *"MUST NOT close a gap while leaving its residual undisclosed"*, and
it sits behind a function doc that asserts the opposite (*"there is no interpreter left in
the path to parse anything"*). That is the exact shape CR-02 had in round 9. It is
gaps[0].

**What the round DID close, measured not assumed.** I extracted `src/ui/screens/detail.rs`
at `80bc4c1`, re-implemented `text.rs`'s new interpreter census in Python from the
committed source, and ran it: it reports lines **1693 and 2097** — both pre-fix `sh -c`
sites. The "this census would have caught CR-01" claim is a measurement. A tree-wide grep
for `"-c"` / `"sh"` / `"bash"` / `/bin/sh` under `src/` finds only
`project_creator::execute_hook` (operator-authored hook from the user's own
`config.json`, correctly scoped out) and two fixed-program test fixtures. The shell class
is genuinely gone from both Sessions-tab spawn paths, including the `n` new-session key
the plan never named (`detail.rs:2222-2225`, argv + `current_dir`, no untrusted element).

### The three new censuses — each exercised against a fixture, not read

| Census | Claim | My measurement | Verdict |
|---|---|---|---|
| `text::tests::no_executable_line_under_src_hands_an_interpreter_an_interpolated_program` | reports **any** executable line naming an interpreter and interpolating into what it is handed | Re-implemented and run against CR-01-shaped fixtures with N comment lines between `"-c"` and `&format!`: **0→reported, 4→reported, 10→reported, 11→reported, 12→MISSED, 13→MISSED, 20→MISSED.** Cause read directly at `src/text.rs:1750-1755`: `taken += 1` precedes the comment check, so a skipped comment still spends join budget. Separately: `has_an_unclosed_delimiter`'s doc (`:1707-1712`) says its method-chain price is *"stated as a residual on the census itself"*; the census's residual block (`:1800-1815`) states three residuals and that is not one of them. | **FAILED as a completeness claim** — gaps[1] |
| `driver::tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition` | an executable `sanitize_render_line` call in those two files appears **only** inside a composition | Re-implemented `logical_lines` + `composition_census` and ran the two-arm `match` fixture (one arm composed, its sibling not): **`census hits: []`** — the composed arm launders the un-composed one, because both land in one logical unit. Also read directly: `composition_census` truncates at the first `#[cfg(test)]` LINE (`:2168-2172`) while NON-VACUITY 2 counts the needle over the WHOLE file (`:2252-2268`), and a mid-file `#[cfg(test)]` helper is already live in this tree at `detail.rs:5342`. | **FAILED as a completeness claim** — gaps[2] |
| `ui::tests::every_render_site_under_ui_composes_both_classes` | every render site under `src/ui/` composes both classes | Measured the needle at HEAD across all 16 `.rs` files under `src/ui/`: **7 executable `display_identity(` occurrences total, 1 of them inside the `render_escape_guard.rs` exemption — so 6 executable lines in 2 of 16 files drive the assertion** (`driver.rs` 2, `driver_confirm.rs` 4). `detail.rs` (7 463 lines) and `normal.rs` (2 167 lines) contribute zero. A site that escapes NOTHING carries no needle and is invisible; the residuals block (`:346-356`) does not disclose this. Coverage shrinks as the conversion succeeds. | **FAILED as a completeness claim** — gaps[3] |

**I looked for a live leak behind gaps[3] and did not find one.** I sampled render sites
*outside* the census's needle — 63 `Span::raw`/`Span::styled` sites under `src/ui/` whose
argument is neither a literal nor an obvious escape — and hand-traced the risky ones:
`normal.rs:734` (`phase_cell` ← `render_for_terminal`), `:800`/`:808` (`alias_read` ←
`render_for_terminal`), `driver.rs:841` (`row` ← `shown_capped(goal)`), `:883` (`command`
← `shown_capped`), `detail.rs:2887`/`:3064` (`shown(&event.description)`), `:2957`
(`line_text` built from `shown(&phase.number)`/`shown(&phase.name)`), `:4297`
(`EditBuffer::shown()`), `:4346-4347` (authored dropdown options and an authored marker),
`driver_confirm.rs:441`. Every one is either an authored `&'static str`/const or escaped
one to three lines upstream. I also verified `detail.rs`'s local `shown` helper is
`crate::text::render_for_terminal(..)` — both classes, not one (`detail.rs:77-79`). So
gaps[3] is a false completeness claim with an undisclosed residual, **not** a live leak.

### Round 10's own must-have truths — 36 of 40 verified

| Plan | # | Truth (abbreviated) | Status | Evidence |
|---|---|---|---|---|
| 21-27 | 1 | CR-01 closed by deleting the shell; no parser left to make a metacharacter code | **FAILED** | Shell class gone (verified). "No parser left" is false: `claude`'s own option parser remains and `--resume` takes an optional value. gaps[0]. |
| 21-27 | 2 | Proven at the argv by a control over a hostile id: arity, byte-identity, no interpreter, no `-c` | VERIFIED *(coincidental-reliance: fixture-only)* | Read `detail.rs:7304-7373`; four assertions present and green. Its fixture set is drawn from the same enumeration the doc's claim is — see `coincidental_reliance_items`. |
| 21-27 | 3 | Round 9's false rationale corrected with the sentence it corrects quoted verbatim | VERIFIED | `detail.rs:580-596` and the in-line comment at `:1798-1815` both quote round 9's sentence verbatim and name the sink kind. (The accompanying "no parser left" clause is scored under truth 1.) |
| 21-27 | 4 | `as_raw_for_logic_only`'s doc names the third question and states the argv-never-a-program-string rule | VERIFIED | Read; the rule is stated and the census below is what checks it. |
| 21-27 | 5 | The rule is CHECKED by a committed census reporting **any** such executable line; needle assembled at runtime; observed RED by planting | **FAILED** | Needle-splitting and the planted red are real (I reproduced the red in kind against pre-fix `detail.rs`: sites 1693, 2097). "Any" is false — 12 comment lines defeat it, measured. gaps[1]. |
| 21-27 | 6 | `terminal_program_separator` table + tests over every `find_terminal` candidate | VERIFIED | `detail.rs:569-575` (table), `:7382-7421` and `:7428-7461` (two controls), both green. See WARNINGS for the dead assertion (IN-02) and the missing cwd column (WR-06). |
| 21-27 | 7 | WR-01 closed: six absences certified, each with a presence control arm | VERIFIED | `text.rs:2236+` asserts `Display`, `AsRef<str>`, `Into<Cow>`, `Deref<Target=str>`, `Borrow<str>`, `Serialize` absent, plus `String` presence arms. Read directly. |
| 21-27 | 8 | Every mechanism observed RED before green, red committed verbatim | VERIFIED | Three reds quoted verbatim in the docs/SUMMARY; I independently reproduced the census red in kind. |
| 21-27 | 9 | The four pre-existing clippy lints untouched, MEASURED | VERIFIED | My run: exactly four (3× `bool_assert_comparison`, 1× `cmp_owned`). Neither owning file appears in the diff. Lib gate exits 0. |
| 21-28 | 1 | CR-02 closed AT THE TYPE — `DriverOutputLine::text` is `Untrusted` | VERIFIED | `mod.rs:475`; `push_record` (`:566-583`) is the only construction site in the tree (grep confirms); `driver.rs:1785` renders `.shown()`. |
| 21-28 | 2 | The composition is proven EQUAL to the one it replaces | VERIFIED | `the_wrapped_line_composition_equals_shown_capped` (`driver.rs:2346+`) present and green. |
| 21-28 | 3 | Injection rows and dry-run preview closed at the render, and the call-vs-type difference disclosed | VERIFIED | `driver.rs:1733` and `:1127` both `shown_capped`; the limits block states the direction. |
| 21-28 | 4 | The probe hole is CLOSED, not re-disclosed; arrival asserted per state | VERIFIED | `render_escape_guard.rs:1099/1116/1125/1601`; negative controls at `:2981-3053` empty each fixture and assert the state stops arriving. All green. |
| 21-28 | 5 | `sanitize_render_line`'s false composition claim corrected with the falsified text quoted | VERIFIED | `driver.rs:612` quotes the falsified sentence verbatim. |
| 21-28 | 6 | WR-06 closed with a fixture that can go RED for it | VERIFIED *(premise measured FALSE and corrected in the record)* | `21-28-SUMMARY.md:258` discloses that `render_disclosure` draws `path` from authored `&'static str`s in `DISCLOSED_PROMPT_INPUTS` and `digest` from hex, so **no fixture could go red there** — the plan's premise, not the execution, was wrong. The composition was added anyway as defence-in-depth and the record says so. Counted verified because the round measured its own premise and corrected it rather than manufacturing a green; this is the behaviour the phase exists to produce. |
| 21-28 | 7 | IN-01 closed: an executable call to `sanitize_render_line` in those two files appears **only** inside a composition | **FAILED** | Reproduced three blind spots with the census's own algorithm. gaps[2]. |
| 21-28 | 8 | SAFE-07 boundary reconfirmed (backstop) | VERIFIED | Explicit evidence, not inference: `tests/spawn_seam_guard.rs` 15/0 in my own run, file absent from the round-10 diff. |
| 21-28 | 9 | SAFE-07 precision reconfirmed (backstop) | VERIFIED | Same run plus `driver_injection_corpus`'s 13 active structural pins green; the round-10 diff adds no new channel (no `src/driver/`, no `tests/`). |
| 21-28 | 10 | Four plants, four reds | VERIFIED | Reds quoted; the SUMMARY's `git status` claim is qualified precisely rather than overstated (`21-28-SUMMARY.md:201`), which I read as honest rather than as a shortfall. |
| 21-28 | 11 | Clippy four, measured | VERIFIED | As above. |
| 21-29 | 1 | WR-05 closed by converting every named render site to `render_for_terminal` | VERIFIED | All nine named files appear in the diff; the census reports zero non-exempt sites. |
| 21-29 | 2 | The claim becomes a committed control over `src/ui/` | **FAILED** | Mechanism real and green; the completeness its name asserts is not checked. 6 executable lines in 2 of 16 files, measured. gaps[3]. |
| 21-29 | 3 | The ratatui dependency-behaviour mitigation written down with its direction | VERIFIED | `ui/mod.rs:326-345`, including "under-detection if the dependency changes, silent", pointing at the standing `deferred-items.md` obligation. |
| 21-29 | 4 | WR-07 closed as an ordering defect, with the reviewer's panic claim refuted in the record | VERIFIED | `backlog.rs:155-157` is `f64::total_cmp` over a finite-filtered key, no fallback arm; the doc quotes pass 10's refutation verbatim with attribution. See WARNINGS for the remaining tie residual. |
| 21-29 | 5 | WR-08 closed by asserting per character, with a two-invisible-character fixture | VERIFIED | `LOOK_ALIKE_PAIRS` is now `[(&str,&str); 7]` including `d\u{200b}emo\u{00ad}`; `src/error.rs` changed accordingly; green. |
| 21-29 | 6 | Conversions proven behaviour-preserving in both directions | VERIFIED | `the_conversion_is_a_no_op_on_clean_values_and_is_not_on_control_values` (`ui/mod.rs:537`), green. |
| 21-29 | 7 | Three plants, three reds | VERIFIED | Reds quoted verbatim, including the 23-site pre-conversion red and the `help.rs` plant. |
| 21-29 | 8 | Clippy four, measured | VERIFIED | As above. |
| 21-30 | 1 | CR-03 closed AT THE TYPE — `EditBuffer` over `Untrusted` | VERIFIED | `mod.rs:787,909`; `detail.rs:4267` renders `.shown()`. The re-diagnosis (a laundered escape, not a missing one) is correct and is what makes the type-level fix the right one. |
| 21-30 | 2 | `EditBuffer` supports what the edit surface needs and nothing that re-opens the raw path; char ops, not byte ops | VERIFIED *(with a WARNING)* | Read `mod.rs:797-846`: seed, `push_char`, `pop_char`, `clear`, `shown`, one named raw take. Character-wise. **Warning: the trait-absence claim in its doc is prose-only — `rtk proxy grep -c "implements_" src/ui/screens/mod.rs` returns 0** (WR-04). |
| 21-30 | 3 | What the operator types is what is persisted, byte-identical | VERIFIED | `what_the_operator_types_is_what_is_persisted` (`mod.rs:1712-1740`) green; `detail.rs:2803` is the single take site. |
| 21-30 | 4 | The probe reaches the state, asserting a branch token first | VERIFIED | `render_escape_guard.rs:1618-1635` sets `defaults_editing`; `:3071-3123` is the negative control. Green. |
| 21-30 | 5 | WR-02 closed: a third disposition value is not expressible; no screen file touched | VERIFIED | `RenderDisposition` is a crate-private two-variant enum (`mod.rs:137-165`); the two constants keep their names (`:175-181`); all eleven `adjudicate_screen!` sites unchanged. |
| 21-30 | 6 | WR-03 closed: `adjudication_reason` gains readers and a control | VERIFIED | Readers at `render_escape_guard.rs:2337/2691/2745/2776/2824`; control `every_adjudication_reason_is_non_empty_and_names_values_not_verdicts` at `:2377`. The record honestly discloses that one reason DOES use a forbidden word (`deferred-items.md:794`) — the finding WR-03 predicted, reported rather than quietly reworded. |
| 21-30 | 7 | IN-02 closed with `chars().count()`, no dependency added | VERIFIED | `detail.rs:4283`; `git diff 80bc4c1..HEAD -- Cargo.toml Cargo.lock` is empty (I ran it). |
| 21-30 | 8 | DRIVE-04 boundary reconfirmed (backstop) | VERIFIED | Explicit evidence: `driver_escalation_cap` 8/0, my own run; file absent from the diff. |
| 21-30 | 9 | DRIVE-04 precision reconfirmed (backstop) | VERIFIED | Same run, same suite. |
| 21-30 | 10 | The record carries the disclosure that was missing, the corrected carrier table, the ten-item triage, WR-07's refutation, criterion 4 verbatim | VERIFIED | `deferred-items.md:661-980` — all present, append-only, dated, quoting what they correct. `COVERAGE.md` exists with a reasoned no-external-API declaration and correctly refuses to fabricate matrix rows for the `claude` subprocess seam. |
| 21-30 | 11 | Four plants, four reds (including a compiler error for the third disposition) | VERIFIED | Quoted in the SUMMARY. |
| 21-30 | 12 | Clippy four, measured | VERIFIED | As above. |

**Round-10 must-haves: 36/40 VERIFIED, 4 FAILED (21-27 t1, 21-27 t5, 21-28 t7, 21-29 t2).**
All four failures are the same species and it is this phase's named species: **a completeness
claim wider than the control that certifies it.** Three of the four are in mechanisms this
round BUILT to certify completeness.

### Combined score

- ROADMAP criteria: **4/5** verified, 1 behavior-unverified, 0 failed (unchanged from passes 9 and 10)
- Round-10 must-have truths: **36/40** verified, 4 failed
- **Total: 40/45 must-haves verified**

### Requirements Coverage (pass 11 delta)

| Requirement | Status (pass 10 → pass 11) | Evidence |
|---|---|---|
| DRIVE-01 | SATISFIED → still SATISFIED | Criteria 1 and 2 regression-checked; `src/driver/` absent from the round-10 diff. |
| DRIVE-03 | SATISFIED → still SATISFIED | Criterion 1's machinery and the text alphabet primitives unchanged (diff-grepped per signature). |
| DRIVE-04 | SATISFIED → still SATISFIED | `driver_escalation_cap` 8/0, my own run, both backstop directions. |
| SAFE-07 | NEEDS HUMAN + 2 gaps → **NEEDS HUMAN, both pass-10 gaps CLOSED, 3 new completeness gaps** | Criterion 4's behavioural half unchanged and permanently human-only. CR-02 and CR-03 (pass 10 gaps[1], gaps[2]) are closed at the type and confirmed by direct read. The three census completeness failures (gaps[1..3]) fall in this requirement's render/execution-honesty scope. |
| SAFE-08 | SATISFIED → still SATISFIED formally; **CR-01' is a related-but-distinct BLOCKER** | `parse_action`'s enum lookup is unchanged and still refuses every out-of-enum action name; `driver_refusal_record` 9/0. CR-01' is a human-triggered TUI action over a `/proc`-scraped id, not the model's chosen action — the scoping settled at pass 10 and re-affirmed here. It is gaps[0] on its own merits. |

**No orphaned requirements.** I took the union of the `requirements:` field across all 30
plans: exactly `{DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}`, matching
REQUIREMENTS.md's phase-21 mapping (lines 152-167). REQUIREMENTS.md is untouched for the
eighth consecutive round.

### Behavioral Spot-Checks (pass 11)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace suite (run ONCE, `--no-fail-fast`) | `rtk proxy cargo test --all-targets --no-fail-fast` | 33 binaries green; `driver_reattach` 1 passed / 2 failed (the two documented flakes); 13 ignored total (10 corpus + 3 `driver_kill_startup`) | PASS (with known flake) |
| Library tests | (same run) | 1103 passed / 0 failed | PASS |
| Escalation cap, both directions | `--test driver_escalation_cap` | 8 passed / 0 failed | PASS |
| Out-of-enum action refused | `--test driver_refusal_record` | 9 passed / 0 failed | PASS |
| Spawn-seam structural boundary | `--test spawn_seam_guard` | 15 passed / 0 failed | PASS |
| Injection corpus (structural half) | `--test driver_injection_corpus` | 13 passed / 0 failed / **10 ignored** | PASS (behavioural half SKIP → human) |
| Clippy, project gate (lib) | `rtk proxy cargo clippy -- -D warnings` | exit 0 | PASS |
| Clippy, `--all-targets` | `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly 4 lints, same 2 kinds, same 2 files | PASS (documented deferral) |
| No dependency added | `git diff 80bc4c1..HEAD -- Cargo.toml Cargo.lock` | empty | PASS |
| Interpreter census vs. the pre-fix tree | Python re-implementation over `git show 80bc4c1:src/ui/screens/detail.rs` | reports 1693, 2097 | PASS |
| Interpreter census vs. a comment-padded CR-01 fixture | Python re-implementation, N = 0…20 | MISSED at N ≥ 12 | **FAIL** → gaps[1] |
| driver.rs composition census vs. a two-arm `match` fixture | Python re-implementation | `census hits: []` | **FAIL** → gaps[2] |
| src/ui census needle coverage at HEAD | script over all 16 `.rs` files under `src/ui/` | 7 executable occurrences; 6 non-exempt, in 2 files | **FAIL** → gaps[3] |

### Anti-Patterns Found (pass 11, incremental)

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/ui/screens/detail.rs` | `619-630` | Argument injection (CWE-88): untrusted `/proc`-scraped value is the last argv element after an optional-value option, with no `--` end-of-options marker | **Blocker** | gaps[0]. A same-user process named `claude` with cmdline `claude --resume --dangerously-skip-permissions` makes the operator's resume spawn that flag in the attacker's cwd. Measured against the real CLI's `--help`. Undisclosed in every round-10 artifact. |
| `src/ui/screens/detail.rs` | `598-608`, `7270-7293` | Completeness claim ("no parser left") whose certifying fixture set is the same enumeration the claim is drawn from | **Blocker** | gaps[0]'s second half — the reason no committed control goes red for it. |
| `src/text.rs` | `1750-1755` | Comment lines consume the join budget before being skipped | **Blocker** | gaps[1]. Measured: 12 comment lines silence the census on the exact construction it exists to catch. |
| `src/text.rs` | `1707-1712` vs `1800-1815` | A doc cross-references a residual that the block it points at does not contain | Warning | gaps[1]'s second half. Prose pointing at prose. |
| `src/ui/screens/driver.rs` | `2091-2145`, `2159-2185`, `2252-2268` | Over-joining across `match` arms; non-vacuity guard counts a different set than the census; `#[cfg(test)]` line truncation is a silent kill switch | **Blocker** | gaps[2]. All three reproduced with the census's own algorithm. |
| `src/ui/mod.rs` | `196-231`, `357-408` | A control named for a property it does not check, driven by 6 executable lines in 2 of 16 files, whose coverage shrinks as the conversion succeeds | **Blocker** | gaps[3]. No live leak found behind it (63-site sample hand-traced), but the residual is undisclosed. |
| `src/ui/screens/mod.rs` | `750-757` | `EditBuffer`'s trait-absence claim is prose-only — the exact standard 21-27 applied to `Untrusted` in the same round | Warning | WR-04. `grep -c "implements_"` in that file returns 0. Adding `impl Display for EditBuffer` tomorrow restores the laundering path CR-03 closed with every test green. The doc's `buffer.clone()` sentence is also inaccurate: `EditBuffer` derives no `Clone`. |
| `src/state_reader/backlog.rs` | `112-157`, `283-330` | The comparator is total over KEYS; every unusable suffix maps to `0.0`, so tied ELEMENTS keep `read_dir`'s order, which is the very defect the failure message describes | Warning | WR-05. One-line fix: `.then_with(\|\| a.cmp(b))`. Display-ordering only. |
| `src/ui/screens/detail.rs` | `611-616`, `1818`, `2224` | The `cd '<dir>' &&` → `Command::current_dir` half of the CR-01 fix has no control and no per-emulator table column, while the separator half of the same change got both | Warning | WR-06. `gnome-terminal` is a D-Bus-activated client and is one of exactly four probed candidates; failure is silent (session lands in `$HOME`). |
| `src/ui/screens/detail.rs` | `7457` | `assert!(matches!(separator, "-e" \| "--"))` cannot fail — the function is a two-arm match returning those two literals | Info | IN-02. Dead assertion; the load-bearing arm is `candidates.len() == 4` above it. |
| `src/ui/screens/detail.rs` | `1820-1826`, `2226-2232` | "Resumed session {}" is reported on `spawn()` returning, not on the resume working | Info | IN-03. Pre-existing shape, but `terminal_program_separator`'s residual now rests its "LOUD rather than silent" direction on it. |
| `src/ui/mod.rs` | `74-94` vs `106-113` | `EXEMPTIONS` matches whole-file while `WAVE_PENDING`'s doc argues for exact `path:line` pinning | Info | IN-01. Defensible today (the exempt file is `#[cfg(test)]`-gated) but unexplained. |
| `src/text.rs` | `1698-1704` | `interpolates_into_a_string`'s marker set is narrower than its own doc | Info | IN-05, and a contributing cause of gaps[1]. |
| `src/ui/screens/detail.rs` | `1866`, `4024-4079` | `ArchiveDepth::milestone` is the same untyped round trip 21-30 closed for `defaults_text_buffer` | Info | IN-04. Correctly escaped at each of the three render sites and honestly disclosed with its direction; recorded so the last instance of the pattern the round made a defect class is on the record. |

### Deviation self-disclosures by the round itself — checked and credited

Three of the four plans disclosed a shortfall or a corrected premise that no verifier
asked them for. I checked each against the code and credit all three:

1. **21-28 corrected WR-06's premise.** The plan asserted `render_disclosure` draws
   `input.path` and `digest` from the recorded opt-in block in `config.json`. The executor
   measured the live path (`registry::current_prompt_inputs` builds every `path` from
   authored `&'static str`s in `DISCLOSED_PROMPT_INPUTS` and every `digest` from
   `sha256_digest`) and reported that no screen fixture COULD go red there. That is the
   plant-and-observe discipline catching a bad premise before it shipped as false green.
2. **21-29 disclosed its S1 acceptance criterion as NOT met** (`21-29-SUMMARY.md:577`):
   both delivered spot-checks are `.planning/`-derived; the input-echo half was not
   delivered because the fix would have required editing a file fenced to a parallel
   worktree. Reported, not absorbed. Scored as an INFO shortfall, not a gap.
3. **21-30 re-diagnosed CR-03** from "an unescaped render" to "an escape laundered by a
   round trip", which is what made the type-level `EditBuffer` fix the correct one rather
   than a second `shown()` call at the popup.

### Gaps Summary (pass 11)

**Round 10 closed all three of pass 10's gaps, and closed them at the strongest available
level.** CR-02 and CR-03 are closed AT THE TYPE — the compiler, not a reviewer, now
enforces the escape on the pane that shows the model's own prose and on the config-edit
popup — and the nine-round-old probe hole behind CR-02 is closed with negative controls
that empty each fixture and assert the state stops arriving, which is the specific
counter-measure to "a populated cache the render never reads". WR-01 is closed by raising
the certificate to the claim rather than lowering the claim to the certificate. The record
work is genuine: `deferred-items.md` carries the disclosure round 9 omitted, a corrected
carrier table, a complete ten-item triage, and criterion 4 re-surfaced verbatim with no
work claimed against it for the third consecutive round.

**But the round's four new certifying mechanisms have measured blind spots wider than the
residuals they disclose, and one of them hides a live defect.** I did not infer any of
this: I re-implemented three census algorithms from their committed source and ran them
against fixtures, and I measured the fourth's needle coverage across all sixteen files it
walks. Every one of the four failures is the same species this phase has been fighting for
eleven passes — **a completeness claim wider than the control that certifies it** — and
three of the four are in mechanisms this round built specifically to end that species.

**The one that is not merely a claim is gaps[0].** 21-27 deleted the shell interpreter and
then asserted, in the function's own doc, that "there is no interpreter left in the path
to parse anything". `claude`'s option parser is still in the path, `--resume` takes an
optional value (I ran `claude --help` to establish this rather than assuming it), and the
session id is `/proc`-scraped with no validation beyond non-emptiness. The fix converted
CWE-78 into CWE-88. The certifying control cannot see it because its fixture set is drawn
from the same enumeration as the doc's claim — the pass-6/pass-7 pathology, one level
down — and it is disclosed in no round-10 artifact, which is the CR-02 pathology repeating
in the round whose own prohibition forbids exactly that. The blast radius is bounded (a
same-user process named `claude`; no privilege boundary crossed), which is why it is a gap
in a hardening phase rather than a wider vulnerability. The fix is one array element and
four fixtures.

**Criterion 4 is unchanged, permanently agent-unclosable, and 4/5 remains the correct
outcome.** Round 10 correctly did zero work against it.

### Recommendation

**One narrow round 11, closing exactly what pass 11 MEASURED, nothing more:**

1. **Fix gaps[0] (CR-01').** Insert `"--"` before the session-id argv element (or refuse a
   non-conforming id at `read_session_id` and report the refusal). Extend
   `hostile_session_ids()` with `-h`, `--dangerously-skip-permissions`, `--print` and a
   bare `-`. Add the STRUCTURAL assertion — the untrusted element is immediately preceded
   by `"--"` — so the absence of the separator goes red, not merely the presence of a
   quote. Correct the doc and disclose the class in `deferred-items.md` with its direction.
2. **Fix gaps[1].** Move `taken += 1` after the comment check. Write the method-chain
   residual the other doc already promises. Widen `interpolates_into_a_string`.
3. **Fix gaps[2].** Reject on the innermost call, compute the non-vacuity total over the
   same slice the census scans, and truncate at the test MODULE.
4. **Fix gaps[3].** Cheapest honest option: rename to
   `no_display_identity_call_under_ui_stands_outside_a_composition` and add the missing
   residual. The version that makes the current name true needs a per-file needle
   non-vacuity guard plus widened `render_escape_guard` probe states.
5. **Cheap and worth it:** give `EditBuffer` the trait-absence control `Untrusted` got in
   the same round (WR-04), and add `.then_with(|| a.cmp(b))` to the backlog comparator
   (WR-05).
6. **Criterion 4 still needs a human and nothing else** — unchanged from every prior pass.

---

_Verified: 2026-08-27_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `feb37ee` · eleventh verification pass · pass 10 preserved above, pass 9 at `f442881`, pass 8 at `f1a9d0d`, pass 7 at `84143bb`_

---

## PASS 12 ADDENDUM (2026-08-27, verification pass 12, after round 11: plans `21-31`..`21-34`)

**Scope of this pass.** Round 11 (`21-31`..`21-34`) executed in 2 waves since pass 11 and
closed pass 11's four gaps plus WR-04, WR-05 and the carried S1 item. Base commit
`343c408` (pass 11's own recording commit), HEAD `ac5267a`. This pass independently
re-derives each of the four gap closures by reading the fixed code directly and by
running the single named test that exercises each fix — not by trusting `21-31`
through `21-34`'s SUMMARYs, `deferred-items.md`'s own narrative, or the fact that the
merged tree is green. Every count- and presence-bearing command was run through `rtk
proxy`.

### ROADMAP Success Criteria — reconfirmed, no regression

| # | Truth | Status | Evidence (pass 12) |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | VERIFIED (regression-checked) | `git diff --stat 343c408..HEAD` names no file under `src/driver/` and no file under `tests/` except the comment-only `tests/driver_reattach.rs`. Ran the full workspace suite myself: 1424 passed / 0 failed / 13 ignored, 34 binaries — matches the orchestrator's pre-measured gate exactly. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | VERIFIED (regression-checked) | `src/driver/run.rs` and `src/driver/mod.rs` absent from the round-11 diff. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | VERIFIED (reconfirmed, ran myself) | `cargo test --test driver_escalation_cap`: 8 passed / 0 failed. File untouched by the round-11 diff. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | PRESENT_BEHAVIOR_UNVERIFIED (unchanged, permanently agent-unclosable by explicit user decision) | `cargo test --test driver_injection_corpus`: 13 passed / 0 failed / 10 ignored, identical to passes 9-11. `tests/driver_injection_corpus.rs` absent from the round-11 diff. **4/5 is the expected and correct outcome — not scored as a failure, per this task's explicit constraint.** |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | VERIFIED (regression-checked) | `parse_action` unchanged (not in the round-11 diff). `driver_refusal_record`: 9 passed / 0 failed, run by me. |

**ROADMAP score: 4/5 verified, 0 FAILED, 1 behavior-unverified — unchanged from passes 9-11. No regression on any of the four VERIFIED criteria.**

### The four pass-11 gaps — each independently re-derived, not trusted

I did not accept `deferred-items.md`'s own account that a gap is closed. For each, I
read the fixed code directly and ran the single named test that is claimed to exercise
it.

| Gap | Fix, read directly | Test run myself | Verdict |
|---|---|---|---|
| gaps[0] (CR-01', CWE-88 argument injection) | `resume_terminal_argv` (`detail.rs:703-715`) fuses the id to `--resume=<id>` in ONE element via `RESUME_OPTION_FUSED_PREFIX`. The falsified doc sentence is corrected in place, quoting itself. `hostile_session_ids()` widened with 10 option-lookalike fixtures, non-vacuity-asserted (`corpus.iter().any(id starts_with '-')` and a shell-metacharacter non-vacuity check). | `ui::screens::detail::tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program` — 1 passed. This test is STRUCTURAL: it asserts the count of `-`-leading argv elements is a CONSTANT independent of the session id across all 28 corpus entries and 5 terminals — a content-independence property, not an enumeration of forbidden bytes. | **CLOSED** |
| gaps[1] (interpreter census comment-budget bug) | `taken += 1` (`text.rs:1810`) now sits AFTER the `next.starts_with("//")` `continue`, confirmed by reading the surrounding loop in full. | `text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments` — 1 passed. Asserts BOTH sides of the boundary: the census still MISSES the CR-01 shape at 12+ EXECUTABLE filler lines (unchanged — the executable window did not widen) and now REPORTS it at up to 100 COMMENT filler lines (previously missed at 12). | **CLOSED** |
| gaps[2] (composition census over-joining) | `occurrence_is_composed` (`driver.rs:2293-2299`) takes the verdict at the innermost call — the text immediately preceding each needle occurrence — rather than over a joined logical unit, so there is no window left to launder across `match` arms. | `ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling` — 1 passed. Uses the exact two-arm `match` fixture pass 11 reproduced the laundering with (`census hits: []` under the old rule); now reported. Also confirmed: `test_module_marker_lines` cuts at a column-zero `#[cfg(test)] mod` marker with an asserted count of exactly one per file (`driver.rs:2495-2506`), closing the silent-truncation defect. | **CLOSED** |
| gaps[3] (`src/ui/mod.rs` false completeness claim) | Test renamed `no_display_identity_call_under_ui_stands_outside_a_composition`. Doc states the measured reach (`MEASURED_REACH` — 8 occurrences, 6 non-exempt in 2 of 16 files) as a checked equality that fails in both directions, discloses the shrinking-coverage property with its direction, and hands the actual completeness claim by name to the sealed `RenderAdjudicated` bound and the `render_escape_guard` behavioural probes. | `ui::tests::no_display_identity_call_under_ui_stands_outside_a_composition` — 1 passed. | **CLOSED, by narrowing rather than by a fourth census — this is the pattern the plan called for and it is what shipped.** |

**No new census was built for any of the four.** Each fix is a repair to the mechanism that already existed plus a narrowed or corrected claim, exactly as each plan's own "PATTERN CALL" section committed to before execution — I checked the pattern calls against the shipped code and found no drift between what was promised and what was built.

### The retired coincidental-reliance item

Pass 11 flagged `21-27` truth 2 (the argv-fix control) as `coincidental-reliance:
fixture-only` — its fixture set was drawn from the same enumeration its doc claimed
completeness over. That control (`the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`)
still exists unchanged, but it now sits alongside
`the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`,
which asserts a STRUCTURAL, corpus-independent property (constant count of
`-`-leading argv elements) rather than a per-fixture absence. I checked this
directly: the structural property generalizes over any hostile string sharing the
"begins with `-`" shape, not only the ones enumerated in the corpus. The
coincidental-reliance flag is therefore retired rather than carried forward —
the property it worried about is now checked by a mechanism that does not share
its weakness. Not re-added to this pass's `coincidental_reliance_items`.

### WR-04, WR-05, and the carried S1 item — each verified directly

- **WR-04** (`EditBuffer`'s trait-absence claim was prose-only): `an_edit_buffer_implements_none_of_the_string_conversions` (`src/ui/screens/mod.rs:1915+`) is a local autoref-specialization probe mirroring `Untrusted`'s pattern, with `String` presence-control arms. Ran myself: 1 passed.
- **WR-05** (the backlog comparator was total over KEYS, letting tied ELEMENTS fall back to `read_dir` order): `backlog_number_ordering` (`src/state_reader/backlog.rs:180-184`) now reads `backlog_sort_key(a).total_cmp(&backlog_sort_key(b)).then_with(|| a.cmp(b))`. Read directly.
- **S1** (a two-direction spot-check on an input-echo screen — carried since `21-29`, declined a second time by `21-30`): `an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone` (`render_escape_guard.rs:3220+`) drives `EnqueueScreen`'s real `Screen::render` with a `TAG_PAIR` fixture (chosen, and the reason stated, because `Paragraph` drops zero-width but preserves tag characters — a zero-width fixture would have passed vacuously). Ran myself: 1 passed. Delivered with zero visibility changes to `src/ui/screens/mod.rs`, as the plan's own measurement (`render_escape_guard.rs` already reaches `super::tests::ctx_with_aliases` as a child module) required.

### The `driver_reattach` flake record — corrected, and the correction's own contradicting measurement disclosed rather than smoothed

`21-34` corrected the stale `--test-threads=1` mitigation claim AT the sentence
that made it (not only in a later section), added a comment-only module note to
`tests/driver_reattach.rs` (diff confirmed by me: 81 insertions, 0 deletions,
`git diff --stat`), and established the correction's completeness with a grep
inventory (`test-threads` + `driver_reattach` across `.planning/`, `tests/`,
`src/`: 388 hits, classified 354 dated-observation / 34 living, of which 7 carried
a standing claim and were corrected).

**What I credit specifically:** `21-34`'s own merged-tree gate run recorded 2
green / 4 red out of 6 isolated `driver_reattach` runs — a WORSE isolated rate
than the "isolated re-runs are reliably green" pattern wave-1's three worktrees
had shown — and this contradiction is written into `deferred-items.md` rather
than absorbed into the more comfortable narrative. A third candidate mechanism
(M3: intra-process thread timing on `isolate_envelope_root`'s `std::env::set_var`
call) is named as a hypothesis, explicitly NOT claimed as proven. My own isolated
run of `driver_reattach` came back 3 passed / 0 failed — consistent with the
now-more-precisely-documented non-determinism, not a regression, not a fix.

**This is not scored as a new gap.** The task's constraint 4 characterizes this
flake as pre-existing at the same rate on base and HEAD; `21-34`'s own additional
measurement sharpens the record of that rate without attributing the flake to
round 11's changes, is explicit that no fix was attempted (per its own
prohibition), and is honest about a finding that complicates its own
predecessor's narrative rather than suppressing it. That is the behaviour this
phase exists to produce, one level removed from the render/escape defects the
rest of the phase is about.

### Anti-pattern scan (round-11 diff)

`git diff 343c408..HEAD -- src/` (5868 insertions, 184 deletions across 21 files at
`80bc4c1..HEAD`; the round-11-only diff at `343c408..HEAD` touches exactly the 8
files declared across the four plans' `files_modified` plus 3 `.planning/`/`tests/`
files under `21-34`) was grepped for `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER`:
**zero hits.** No stub returns, no hardcoded-empty renders introduced. Every plan's
declared `files_modified` list matches the actual diff exactly — no wave-conflict,
no edit outside a plan's declared fence.

### Requirements Coverage (pass 12 — unchanged from pass 11)

| Requirement | Status | Evidence |
|---|---|---|
| DRIVE-01 | SATISFIED | Criteria 1, 2 regression-checked; `src/driver/` absent from the round-11 diff. |
| DRIVE-03 | SATISFIED | Criterion 1's machinery unchanged; alphabet primitives untouched. |
| DRIVE-04 | SATISFIED | `driver_escalation_cap` 8/0, my own run; file absent from the round-11 diff. |
| SAFE-07 | NEEDS HUMAN, behavioural half unchanged; all four render/execution-honesty completeness gaps within this requirement's scope now CLOSED | Criterion 4 permanently human-only. gaps[0]-[3] all independently reproduced closed, above. |
| SAFE-08 | SATISFIED, and CR-01' (gaps[0], the related-but-distinct BLOCKER pass 11 opened) is now CLOSED | `parse_action` unchanged; `driver_refusal_record` 9/0. The Sessions-tab resume fusion closes the argument-injection defect pass 11 found in the mechanism 21-27 built. |

**No orphaned requirements.** Union of `requirements:` across all 34 plans:
exactly `{DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}`, matching
REQUIREMENTS.md's phase-21 mapping. REQUIREMENTS.md untouched for the ninth
consecutive round (`git diff --stat 343c408..HEAD -- .planning/REQUIREMENTS.md`
empty, run by me).

### Behavioral Spot-Checks (pass 12)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace suite (run ONCE, `--no-fail-fast`) | `rtk proxy cargo test --workspace --no-fail-fast` | 1424 passed / 0 failed / 13 ignored, 34 binaries; `driver_reattach` came back 3/3 green this run | PASS |
| gaps[0] structural fix | `--lib ui::screens::detail::tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program -- --exact` | 1 passed | PASS |
| gaps[1] two-sided boundary | `--lib text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments -- --exact` | 1 passed | PASS |
| gaps[2] laundering fixture | `--lib ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling -- --exact` | 1 passed | PASS |
| gaps[2] full census, re-run | `--lib ui::screens::driver::tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition -- --exact` | 1 passed | PASS |
| gaps[3] narrowed + pinned census | `--lib ui::tests::no_display_identity_call_under_ui_stands_outside_a_composition -- --exact` | 1 passed | PASS |
| WR-04 trait-absence probe | `--lib ui::screens::tests::an_edit_buffer_implements_none_of_the_string_conversions -- --exact` | 1 passed | PASS |
| S1 input-echo spot-check | `--lib render_escape_guard::tests::an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone -- --exact` | 1 passed | PASS |
| Escalation cap | `--test driver_escalation_cap` | 8 passed / 0 failed | PASS |
| Out-of-enum action refused | `--test driver_refusal_record` | 9 passed / 0 failed | PASS |
| Spawn-seam structural boundary | `--test spawn_seam_guard` | 15 passed / 0 failed | PASS |
| Injection corpus (structural half) | `--test driver_injection_corpus` | 13 passed / 0 failed / 10 ignored | PASS (behavioural half SKIP -> human) |
| Clippy, project gate (lib) | `rtk proxy cargo clippy -- -D warnings` | exit 0 | PASS |
| Clippy, `--all-targets` | `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly 4 lints, same 2 kinds, same 2 files | PASS (documented deferral) |
| No dependency added | `git diff 343c408..HEAD -- Cargo.toml Cargo.lock` | empty | PASS |
| `tests/driver_reattach.rs` diff is comment-only | `git diff --stat 343c408..HEAD -- tests/driver_reattach.rs` | 81 insertions(+), 0 deletions | PASS |
| REQUIREMENTS.md / driver_injection_corpus.rs untouched | `git diff --stat 343c408..HEAD -- .planning/REQUIREMENTS.md tests/driver_injection_corpus.rs` | empty | PASS |

### Gaps Summary (pass 12)

**All four of pass 11's gaps are closed, and each closure was independently
re-derived rather than trusted.** I read the fixed code directly for each of the
four and ran the single named test claimed to exercise it, rather than relying on
`deferred-items.md`'s own narrative or the fact that the merged tree is green.
gaps[0]'s fix is stronger than a literal reading of pass 11's own recommendation
would have produced — the planner measured that pass 11's suggested `--`
separator fix would have silently deleted the resume capability, and chose
fusion instead, with the measurement (`claude --resume -- <uuid>` returns the
same error as a hostile input) quoted in the plan and reproduced in the record.
gaps[1]-[3] are each closed by the pattern this phase's own eleven-pass arc
converged on: a bounded repair to the existing mechanism plus a narrowed,
honestly-scoped claim — never a new census layered on top of the one that
failed.

**I found no new gap distinct from ROADMAP criterion 4.** The round-11 diff
touches exactly the files each plan declared, contains no debt markers, adds no
dependency, leaves REQUIREMENTS.md and the injection corpus untouched, and the
merged-tree gate (full workspace suite, both clippy invocations) matches the
orchestrator's independently pre-measured figures exactly. The one item I
scrutinized hardest for a hidden regression — the `driver_reattach` flake
record's self-contradicting measurement — is disclosed rather than smoothed and
is consistent with (not different in kind from) the pre-existing non-determinism
this task's constraints describe; I do not score it as a gap.

**Criterion 4 is unchanged, permanently agent-unclosable, and 4/5 remains the
correct outcome.** Round 11 correctly did zero work against it, confirmed by
`tests/driver_injection_corpus.rs`'s absence from the round-11 diff and its
unchanged 13/0/10-ignored count.

### Recommendation

**No further gap-closure round is needed for this phase.** Every must-have this
pass could verify is VERIFIED; the phase's sole open item is ROADMAP criterion
4, which needs a human with an authenticated Claude subscription to run
`cargo test --test driver_injection_corpus -- --ignored --nocapture` and record
the result, per the standing item in `deferred-items.md`. This is a `human_needed`
status, not `gaps_found` and not `passed` — the distinction the status vocabulary
exists to draw. This phase should not be re-verified again on the strength of
further agent-executed rounds; the next legitimate state change requires a human
running the ten ignored tests.

---

_Verified: 2026-08-27_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `ac5267a` · twelfth verification pass · pass 11 preserved above at `feb37ee`, pass 10 at `c9345a1`, pass 9 at `f442881`, pass 8 at `f1a9d0d`, pass 7 at `84143bb`_
