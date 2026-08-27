---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-27T00:00:00Z
head: c9345a1
status: gaps_found
score: 30/33 must-haves verified (4/5 ROADMAP success criteria; 0 ROADMAP criteria FAILED, 1 behavior-unverified; 3 round-9 render/execution-surface completeness claims FAILED)
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 24/26 must-haves verified (4/5 ROADMAP success criteria)
  gaps_closed:
    - "pass-9 gaps[0] (`the_screen_renders_identity_escaped` intermittently RED for DetailScreen [GitHistory tab]): CLOSED DETERMINISTICALLY. `probe_ctx` now populates `git_entries` with a hostile `GitLogEntry` (21-23), so the tab renders its List branch on every run instead of its empty branch on ~1/80. I re-ran the single named test five consecutive times: 5/5 green, ~2.6s each, no variance. Pass 9's dichotomy — a real state-dependent leak vs. a nondeterministic probe — resolves to reading (a): it was a real leak, reached only when the fixture happened to populate `git_entries`, now reached every run and escaped via `shown()` at the git-history render sites the compiler named after `GitLogEntry`'s four fields were retyped to `Untrusted`."
    - "pass-9's TAG_PAIR hand-maintained-index coincidental-reliance flag: CLOSED. `survives_a_rendered_buffer` is now asserted as a checked PRECONDITION — `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell` (`render_escape_guard.rs:1852-1878`) renders the tag pair and a zero-width pair through both `ProbeSink::Paragraph` and `ProbeSink::ListItem` and asserts the class-membership/cell-survival split directly, so a `LOOK_ALIKE_PAIRS` reorder that replaced the tag pair with a zero-width one would fail this test rather than silently going vacuous. I ran it: green."
    - "pass-9's Judgment 3 ratatui generalisation (\"Buffer drops zero-width graphemes\") was ITSELF corrected this round, append-only, in `deferred-items.md`'s 2026-08-27 entry: the drop is a `Paragraph` property, not a `Buffer` property — `Block::title` and `ListItem` PRESERVE the whole zero-width class. That correction is what let 21-23/21-25 find CR-04 and the five `.planning/`-derived `ListItem`/`Block::title` leaks pass 9's Judgment 5 first named. I did not re-derive the per-widget table myself this pass (no scratch crate), but it is internally consistent with the one column pass 9 DID re-derive (`Paragraph`: zero-width dropped, tag block survives) and is corroborated by `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell`, which is a committed, currently-green control over exactly that split."
    - "The five `.planning/`-derived render sites pass 9's Judgment 5 named raw (`detail.rs:2882/3349/3416/3438/3444`) are now escaped: `BacklogItem`, `ArchiveFile`, `PhaseArchive`, `BrowserEntry`, `ClaudeSession` carry `text::Untrusted`, and every render site the compiler named resolves to `shown()` or `as_raw_for_logic_only()` with a one-line reason. I read all five sites; each now calls `shown(...)` into the `Span`/`ListItem`/`Block::title`."
    - "The multibyte session-id truncation panic (`[..8]` byte slice on a `String`) is closed: `shorten_session_id` (`detail.rs:114-118`) now does `raw.chars().take(SESSION_ID_DISPLAY_CHARS).collect()`, a `char` operation. Read directly; no panic path remains."
    - "REQUIREMENTS.md untouched for the SEVENTH consecutive round: `git log --oneline -3 -- .planning/REQUIREMENTS.md` still ends at `0c4f712`."
  gaps_remaining: []
  regressions: []
gaps:
  - truth: "Round 9's own central claim (21-23 must-have truth 1) — that a value crossing this system's trust boundaries cannot reach a rendered OR executed sink unescaped because the carrier withholds every conversion, and every remaining raw site is a site the compiler named — is TRUE of the whole surface, not just of the carriers this round retyped"
    status: failed
    reason: "CONFIRMED independently, reading the code directly rather than trusting 21-REVIEW.md's claim. `src/ui/screens/detail.rs:1692-1707` builds a `sh -c` shell command string with `format!(\"cd '{}' && claude --resume '{}'\", session.working_dir.display(), sid.as_raw_for_logic_only())`. Both interpolated values are attacker-influenced by this codebase's own standard: `session_id` is read verbatim out of another process's `/proc/<pid>/cmdline` `--resume` argument (`src/session_detector.rs:96-113`) and `working_dir` is `read_link(\"/proc/<pid>/cwd\")` (`src/session_detector.rs:63`). Neither is validated against `is_identity_char` or any alphabet; a single `'` in either terminates the shell quoting and executes arbitrary code with the operator's privileges. This round's OWN diff touched these exact lines — it changed `sid` to `sid.as_raw_for_logic_only()` and added a comment classifying the value as \"A SUBPROCESS ARGUMENT: the raw id is what `claude --resume` must receive\" — which is the wrong classification: the value is not passed as a discrete argv element, it is spliced into a shell program. `as_raw_for_logic_only()`'s own doc lists 'subprocess arguments' as an accepted use, without distinguishing an argv element (safe, no shell involved) from a shell-command fragment (unsafe) — the two-accessor vocabulary the round built does not currently name the difference, which is exactly why this round's own scoping (compiler-named sites only) could not catch it: the site was never retyped away from being 'a subprocess argument' in the accessor's own words."
    artifacts:
      - path: "src/ui/screens/detail.rs"
        issue: "Lines 1692-1707: unescaped shell command injection via format!-built `sh -c` string, both interpolants attacker-influenced (/proc scrape), neither alphabet-checked. Pre-existing since phase 9 (`867d277`), but this round's diff touched these exact lines and added a security rationale comment that is false."
    missing:
      - "Do not build a shell string. Pass argv directly to `Command::new(&term).args([\"-e\", \"claude\", \"--resume\", sid.as_raw_for_logic_only()]).current_dir(&session.working_dir)`, or if a single `-e` string terminal is genuinely required, shell-quote both values (escape `'` as `'\\''`) in one named helper with its own control, and correct the comment to say 'shell command fragment', not 'subprocess argument'."
  - truth: "Round 9's render-surface completeness claim covers the pane that displays the LLM's own output — the surface a phase titled 'Prompt-Injection Hardening' most needs covered"
    status: failed
    reason: "CONFIRMED by reading the code. `DriverOutput::push_record` (`src/ui/screens/mod.rs:425`) sanitises with `sanitize_render_line`/`strip_terminal_controls` — the CONTROL class only (ESC/C0/DEL/C1) — and stores the result in `DriverOutputLine::text`, a plain `String`, not `text::Untrusted`. `output_line` (`src/ui/screens/driver.rs:1712-1734`) renders `line.text.clone()` straight into `Span::styled(...)`, which reaches a `Paragraph` (`driver.rs:1964`). By this same round's own per-widget measurement (recorded in `deferred-items.md`'s 2026-08-27 entry), `U+E0041` — the tag-block character, this phase's own named 'LLM ASCII-smuggling carrier' — SURVIVES through `Paragraph` (all four measured widget families, in fact). So the invisible-formatting class reaches the pane that shows the agent's own prose. Two further sites on the identical path share the defect: the injection-row render (`driver.rs:1684`, over `InboxMessage.text` read from `inbox.jsonl`) and the dry-run preview (`driver.rs:1088`, over report text the function's own doc says interpolates paths and branch names read from the project). This is NOT covered by any probe: `render_escape_guard.rs`'s `probe_ctx` (confirmed by reading — no `driver_output`, `driver_journal`, or `driver_inbox` population anywhere in the file) leaves the Driver tab's output/journal/inbox at empty defaults, so it renders `no_runs_lines` (the ONE already-composed site) on every probe run. `DriverOutputLine::text` is also absent from `deferred-items.md`'s own 'six carrier types round 9 did NOT retype' disclosure table — so this is not a disclosed residual, it is an undisclosed one, on the single most relevant carrier to this phase's own threat model."
    artifacts:
      - path: "src/ui/screens/driver.rs"
        issue: "Lines 1088, 1684, 1732 (via mod.rs:425-437): three render sites compose only `sanitize_render_line` (control class), never `display_identity`/`shown()`/`render_for_terminal` (invisible-formatting class), on the live output pane, injection rows, and dry-run preview."
      - path: "src/ui/screens/mod.rs"
        issue: "`DriverOutputLine::text: String` (not `text::Untrusted`), and its doc at `sanitize_render_line`'s call sites asserts (falsely, per the sites above) that `driver.rs` and `driver_confirm.rs` compose both classes."
      - path: "src/ui/screens/render_escape_guard.rs"
        issue: "`probe_ctx` never populates `driver_output`/`driver_journal`/`driver_inbox`; LIMIT 1's disclosure names the Defaults edit overlay and `driver_dry_run` as unprobed STATES but does not name the driver output pane or injection rows as unescaped SITES."
    missing:
      - "Compose both classes at the three sites (e.g. `crate::text::display_identity(&sanitize_render_line(..))` at append time, or `shown_capped`-equivalent at each render site), then populate `probe_ctx.driver_output`/`driver_journal`/`driver_inbox` with a hostile fixture so the probe actually exercises this path, and correct `sanitize_render_line`'s doc claim in the same commit."
  - truth: "Round 9's carrier-retype claim — that every render site drawing a value from a retyped struct is now resolved deliberately (`shown()` or `as_raw_for_logic_only()`) — holds for a value that is copied out of a retyped struct into a second, non-carrier buffer"
    status: failed
    reason: "CONFIRMED by reading the code. `entry.value` (a `GsdConfig`/config-entry string) is escaped at the LIST render: `detail.rs:4088`, `Span::styled(shown(&entry.value), val_style)`. But at `detail.rs:1887-1888`, the SAME `entry.value` is copied raw into `cache.defaults_text_buffer: String` when the operator presses Enter to edit it, and at `detail.rs:4146` that buffer is rendered raw: `Span::styled(buffer.clone(), Style::default().fg(Color::White))` inside a `Clear`ed `Paragraph` popup — the edit surface, where the operator is deciding what to write back to disk while reading a value that may not be what it appears to be. `render_escape_guard.rs`'s LIMIT 1 names this exact popup as an unprobed STATE ('draws `defaults_text_buffer` and `entry.key` into a `Clear`ed popup through a code path no probe state reaches') but frames it purely as a coverage gap, not also as a correctness gap — the disclosure does not say the site is unescaped, only unprobed."
    artifacts:
      - path: "src/ui/screens/detail.rs"
        issue: "Line 1887-1888 copies `entry.value` raw into `defaults_text_buffer: String`; line 4146 renders that buffer unescaped in a `Paragraph` inside the popup that IS the round-8/9 render surface this phase exists to close."
    missing:
      - "Escape at the render site: `Span::styled(shown(buffer), ...)` at `detail.rs:4146`, then add a `DETAIL_SUB_STATES` fixture entry with `defaults_editing = Some(idx)` on a `ConfigValueKind::String` row so the state is actually probed rather than only disclosed."
deferred:
  - truth: "General Unicode CONFUSABLES / homoglyph defence in FREE TEXT"
    addressed_in: "Not phase 21 — recommend a new roadmap item"
    evidence: "Unchanged from pass 9. Out of scope by design."
  - truth: "`registry::current_prompt_inputs` absent from BLOCKING_HELPERS; the spawn-gate plan-half comment; dead `PlanStep::rationale`"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, unchanged across rounds 4-9)"
    evidence: "No round-9 plan touched any of the three; deferred-items.md still carries all three with their stated reasons."
  - truth: "Four PRE-EXISTING clippy lints make `cargo clippy --all-targets -- -D warnings` fail"
    addressed_in: "deferred-items.md:200-236, re-measured at round-9 HEAD"
    evidence: "Reproduced by me: `rtk proxy cargo clippy --all-targets -- -D warnings` exits non-zero with the same three `bool_assert_comparison` at `src/browser.rs:131-133` and one `cmp_owned` at `src/project_creator.rs:146`. `git log -1 -- src/browser.rs src/project_creator.rs` still ends at phase 20's `3e948d2`. The stated project gate, `cargo clippy -- -D warnings` (lib only), is exit 0 — I ran it myself."
  - truth: "WR-01 (Untrusted's doc names five absent traits/impls; the certifying control checks three), WR-02 (the sealed-trait doc overclaims in-crate hand-write immunity), WR-03 (adjudication_reason has zero readers), WR-06 (the opt-in disclosure escapes only the control class over authored-default fixtures), WR-08 (the Debug-escape test's concatenated-expected-string trap)"
    addressed_in: "Not closed by round 9 — carried as WARNINGs, not BLOCKERs, because none is a live leak today"
    evidence: "Independently spot-checked WR-01 by reading `text.rs:512-530` (doc: five absences) against `text.rs:1729-1791` (`an_untrusted_carrier_implements_none_of_the_string_conversions`: asserts three — Display, AsRef<str>, Into<Cow>). Confirmed as described. `Untrusted` does NOT currently implement Deref/Borrow<str>/serde, so there is no live leak; the gap is that no control would catch it if one were added tomorrow. WR-02/03/06/08 not independently re-verified this pass beyond reading the cited line ranges; accepted on the reviewer's evidence, which is itself grounded in direct quotation of source."
  - truth: "WR-07 (`parse_backlog_items`'s NaN-producing sort comparator 'panics' per Rust's total-order detection)"
    addressed_in: "PARTIALLY REFUTED by me — recorded here rather than silently dropped"
    evidence: "I built and ran a standalone Rust program (rustc 1.97.1, matching this toolchain) sorting a `Vec<f64>` containing multiple `NaN` values with the exact comparator shape used in `backlog.rs:88-106` (`partial_cmp(...).unwrap_or(Equal)`), at both small (5-element) and larger (2000-element, 1/3 NaN) sizes. Neither run panicked; both produced a silently-wrong order with NaNs interspersed. Rust's stable `slice::sort_by` does NOT panic on a non-total-order comparator on this toolchain — WR-07's specific claim ('Rust's current slice::sort_by detects total-order violations and panics') is not reproducible and is likely incorrect, possibly confusing Rust with Java's TimSort. The underlying issue (a `.planning/phases/999.NaN-x` directory name silently corrupts backlog sort order rather than being a lookup/security issue) is real but is a display-ordering correctness bug, not a DoS/panic, and does not block this phase's goal."
behavior_unverified_items:
  - truth: "A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes (ROADMAP success criterion 4 / SAFE-07)"
    test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed. Every `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload ARRIVED at the model before asserting the command was unchanged; the two suppression controls show the positive/negative `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile and clean arms both really ran."
    why_human: "All ten spawn the real `claude` binary and need an authenticated subscription, so they cannot run inside verification. NO AGENT CAN CLOSE THIS ITEM, and the user has explicitly chosen to leave it tracked in `deferred-items.md`. Counted independently from my own run: `driver_injection_corpus` reports 13 passed / 10 ignored. Round 9 did NO work against it (both round-9 plans that mention it carry an explicit prohibition, and `git diff --stat dfa11c6..HEAD -- tests/driver_injection_corpus.rs` is empty, confirmed by me). Presence and wiring verified for the TENTH consecutive pass; behaviour never exercised by any verification pass of this phase."
human_verification:
  - test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed, with arrival asserted before influence in every class arm and `both_arms_of_every_class_comparison_were_really_executed` green."
    why_human: "Requires an authenticated subscription and spawns the real model binary; cannot run inside verification. This is criterion 4's only behavioural evidence and no verification pass of this phase has ever produced it."
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
