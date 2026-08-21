---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-21T19:00:22Z
status: gaps_found
score: 4/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "round-3-diff review-CR-02 (blank/whitespace --command not refused): command_source now carries `if !command.trim().is_empty()` on the Command arm; confirmed live at HEAD."
    - "round-3-diff review-CR-01 (approval token parsed after decomposition, and skipped entirely on --dry-run): parse_approval_token now runs at src/driver/mod.rs:644, above `if args.dry_run` at :649 and ~180 lines above `decompose`; confirmed live at HEAD, and confirmed the ordering fix by direct code read."
    - "round-2 WR-01 (guard six's header falsely claimed both its named approximations 'fail loud, not silent' as a blanket property): header rewritten to enumerate limits individually; the specific over/under-detection mischaracterization from round 2 is gone (though round 3's review found the rewritten header omits a DIFFERENT, still-live silent gap — see gaps below, this is not the same finding)."
    - "round-2 WR-03 (plan_digest doc named a legacy-record re-check path production cannot take): doc corrected in both src/driver/goal.rs and src/journal/mod.rs (the second copy was undocumented in the plan but found and fixed); confirmed by grep that recheck_approval's two production call sites match what the doc now claims."
  gaps_remaining:
    - "Criterion 1 fails a THIRD consecutive verification cycle. This round's own gap-closure diff (21-11, 21-12) introduced a new Critical of the identical family as the two it closed — round-3 code review's CR-01: a whitespace-only --target-phase is accepted end to end (preview renders cleanly at exit 0; a real run creates a lock file, a run directory, a journal, and a run.json recording target_phase: \"   \") where the byte-identical value is refused for free on --command and --goal. I independently reproduced this against the built binary/library at HEAD, not merely read it in the review."
  regressions:
    - "REQUIREMENTS.md: DRIVE-01 and DRIVE-03 were marked [x]/Complete in commit 760d71f (21-11's own completion commit), while criterion 1 — the ROADMAP success criterion both requirements trace to — was still failing at the time of that commit (the round-3 review that found the still-open CR-01 was committed one commit later, at d4874ec, against the same tree). This is the identical premature-marking mistake commit 828d7cc had to revert earlier in this same phase. Not reverted this time; still wrong at HEAD."
gaps:
  - truth: "A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs (ROADMAP criterion 1)"
    status: failed
    reason: "The two mechanisms named by the last two verification cycles (blank --command, approval-token ordering) are now genuinely fixed and independently reconfirmed. But the anti-recurrence mechanism this round was specifically commissioned to build — a degenerate-payload matrix run through the production CommandSource resolver, meant to make a fourth recurrence of this defect class a test-time certainty — was applied to only two of the resolver's three variants (Command, Goal) and explicitly, by name, exempted the third (Routed / --target-phase) with a justification that is mathematically wrong for 3 of its own 4 enumerated payloads. The result is the third consecutive cycle in which closing the previous cycle's honesty/integrity defect reintroduced a defect of the same family, in the same file, in the same round."
    artifacts:
      - path: "src/driver/mod.rs"
        issue: "`command_source`'s `(None, Some(target_phase)) => Ok(CommandSource::Routed(target_phase.to_string()))` arm (~:400) carries no emptiness/blankness check, unlike its `Command` sibling three lines above it (which gained `if !command.trim().is_empty()` this round) and its `Goal` sibling below it (which has carried the check since 21-07). I reproduced this independently, not from the review: a standalone `#[tokio::test]` built on the project's own `tests/driver_dry_run.rs` fixtures, calling `drive()` with `target_phase: Some(\"   \".to_string())`, `dry_run: true` returned `Ok(())` and rendered `the router would park (router_state_unverified:    ) and the run would stop for` under the same 'complete and honest sequence' preview header. The same call with `dry_run: false` returned `Ok(())` and created `.planning/meta-manager/runs/run.lock`, `.gitignore`, a run directory, `journal.jsonl`, and `run.json` — for an invocation whose command was three space characters. (Reproduction test written and run against HEAD, then deleted; not committed.)"
      - path: "src/driver/mod.rs"
        issue: "The regression test written specifically to make this defect class impossible — `every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload` (~:1875-2000) — explicitly excludes `--target-phase` from its by-name refusal sweep (~:1976-1995: only `--command` and `--goal` are looped over `DEGENERATE`) via a comment claiming `journal::is_plain_path_component` 'returns false for the empty string' as justification. I confirmed by reading `is_plain_path_component` (`src/journal/mod.rs:242-254`): it returns `false` only for the empty string; for `\"   \"`, `\"\\t\"`, and `\"\\n  \\n\"` — 3 of the 4 entries in the test's own `DEGENERATE` array — it returns `true`, because every whitespace string is a single `Component::Normal` equal to itself. The cited test (`a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk`) pins exactly one value, `\"../../../../escaped\"`, and says nothing about blankness. The exemption comment's stated reason is true for 1 of the 4 payloads it is invoked to justify."
    missing:
      - "Add the same `if !target_phase.trim().is_empty()` guard to the Routed arm that the Command arm just gained, with a sibling `(None, Some(_)) => Err(DriveError::NoCommandSource)` refusal arm — the exact fix 21-REVIEW.md already specifies at src/driver/mod.rs's command_source."
      - "Delete the false exemption comment and put --target-phase into the matrix's by-name sweep alongside --command and --goal, so all three positions are asserted refused for every DEGENERATE payload."
      - "Add a `\"   \"` arm to `a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk` so the test the exemption comment cited actually pins what the comment claimed it pins."
      - "Given this is the third cycle in which the anti-recurrence mechanism was scoped too narrowly rather than absent, the next plan should derive the matrix's column set from `variant_name`'s match arms (or an equivalent exhaustive source) rather than a hand-written list of argv positions, so a column cannot be silently omitted the way --target-phase was this round — the same shape of fix `variant_name`'s no-wildcard match already applied to the row axis."
  - truth: "The new guard/doc machinery this round's plans built to prevent recurrence is itself accurate about what it does and does not check"
    status: partial
    reason: "Three guard-accuracy defects in the round's own new machinery, all independently confirmed by direct code reading rather than accepted on the review's word. None is independently exploitable today (no live call site reaches any of the three), and closing them is what stops the NEXT reviewer from having to rediscover them by hand rather than being told."
    artifacts:
      - path: "tests/spawn_seam_guard.rs"
        issue: "Guard six's rewritten header (~:1691-1723) now correctly refuses to make a blanket claim about the guard's failure direction and names three specific limits — an improvement over the round-2 header this replaces. But it still omits a fourth, LIVE limit: `terminal_write_hits` computes `test_region_start(file).unwrap_or(usize::MAX)` and skips every line at or past that boundary to end of file, not just the test module. I confirmed `src/state_reader/mod.rs` has `mod tests {` at line 311 and a genuine production function, `pub fn count_backlog_items`, at line 530 — invisible to the guard — and that `cargo clippy --all-targets` independently flags this exact file with `items_after_test_module`. A `journal.finish(...)` call added to any new production helper placed after line 311 of that file would go unstamped with the guard staying green."
      - path: "tests/spawn_seam_guard.rs"
        issue: "Guard eight (`every_command_source_variant_is_named_only_where_it_is_built_or_matched`, ~:2136-2226), added this round specifically to close the gap that let the round-3-diff CR-02 defect through, names no limitation of its own at all — unlike guard six, rewritten in the same round to name each of its limits individually. I confirmed by reading `enclosing_fn` (`:1130-1136`): it is a nearest-preceding-`fn` backward scan with no brace tracking, so a variant-naming line placed between `fn command_source` and the next `fn` declaration would be silently attributed to `command_source` and allowlisted; and that the three needles (`CommandSource::Command(`, `CommandSource::Routed(`, `CommandSource::Goal(`) do not match `Self::Command(` from inside an `impl CommandSource` block or an imported bare-name construction. No such site exists in the tree today (confirmed by grep), so this is not exploitable, but the header makes no claim it could be checked against."
      - path: "src/driver/mod.rs"
        issue: "`DEGENERATE` (the payload array) and `empty_numbered_entry` (the detector) both define 'blank' with the exact same `str::trim`-based predicate the production guard (`command_source`) uses, and the array's own doc names this coupling as a deliberate virtue ('the test and the code must agree about what blank means'). I confirmed this makes the matrix unable to falsify the guard it is meant to check: a payload outside `char::is_whitespace` — e.g. U+200B ZERO WIDTH SPACE — survives `trim`, so `command_source` resolves it as a real command, and `empty_numbered_entry`'s own `tail.trim().is_empty()` cannot flag the resulting numbered entry as empty. Confirmed by direct code reading of `DEGENERATE` (`:1674`), `empty_numbered_entry` (`:1706-1715`), and the Command arm's guard (`:392`) — all three genuinely share one predicate. Not independently reproduced by spawning the binary with a zero-width-space argument (the review reports having done so and I did not re-run that specific reproduction), but the code-level claim — that the enumeration cannot contain a payload the guard mishandles because both are defined by the same function — is a direct, verifiable reading of the three cited functions and holds."
    missing:
      - "Add a fourth named limit to guard six's header (the round-3 review's suggested wording is ready to use verbatim) stating the boundary excludes everything to end-of-file, not just the test module, and note nothing in the guard bounds it."
      - "Add a limits block to guard eight's header naming the Self::-qualified construction gap, the imported-variant gap, and the enclosing_fn brace-tracking gap; optionally close the cheapest of the three by adding `Self::Command(` / `Self::Goal(` needles."
      - "Break the DEGENERATE/empty_numbered_entry tautology by judging the rendered numbered entry on a broader blank-character set (control chars, zero-width spaces, BOM) than the production `trim` predicate uses, so the matrix can disagree with the guard rather than only ever agree with it, and add at least one non-is_whitespace payload (\\u{200b} or \\u{feff}) to DEGENERATE."
  - truth: "REQUIREMENTS.md accurately reflects verified requirement status for this phase"
    status: partial
    reason: "DRIVE-01 and DRIVE-03 were marked [x]/Complete in the same commit (760d71f) that closed 21-11, while the round-3 code review committed one commit later (d4874ec) against the identical tree found a new Critical that breaks exactly the ROADMAP criterion (criterion 1) both requirements trace to. This repeats, inside the same phase, the exact premature-completion mistake that an earlier commit (828d7cc) had to revert. Not self-caused by 21-11/21-12's code — a documentation/process gap, not a code defect — but it means REQUIREMENTS.md is currently wrong at HEAD in the same direction and for the same underlying reason as before."
    artifacts:
      - path: ".planning/REQUIREMENTS.md"
        issue: "Lines ~83, ~85 (checklist) and ~164, ~166 (traceability table): DRIVE-01 and DRIVE-03 read [x]/Complete. Confirmed by `git show 760d71f -- .planning/REQUIREMENTS.md` that this flip happened in 21-11's own completion commit, and by re-deriving criterion 1's status in this report that it is still ✗ FAILED at HEAD."
    missing:
      - "Revert DRIVE-01 and DRIVE-03 to [ ]/Pending (or Gaps Found, matching DRIVE-04/SAFE-07/SAFE-08's current marking) until a verification pass records criterion 1 as ✓ VERIFIED — a plan-completion commit is not the place a requirement's status is authoritatively decided; that has been this phase's own stated norm in every prior gap-closure summary."
deferred: []
behavior_unverified_items: []
human_verification: []
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-21T19:00:22Z
**Status:** gaps_found
**Re-verification:** Yes — after gap closure (third cycle; fourth verification pass)

## Goal Achievement

This is the **fourth** verification pass and the **third** gap-closure cycle for
this phase. The pattern across all three cycles is the finding, not incidental
to it:

| Cycle | What closed | What the SAME cycle's own diff broke |
|---|---|---|
| 1 → 2 (plans 21-07..21-10) | round-2 CR-01 (false honest-sequence claim for `--goal --dry-run`), CR-02 (invertible FNV digest), WR-01 (unreachable `PlanChanged`), WR-05 (unbounded `target_phase`) | round-3-diff CR-01 (approval token parsed after decomposition, skipped on `--dry-run`), CR-02 (blank `--command` not refused) — found by round-2 code review (`92a4b4f`), confirmed independently by pass-3 verification |
| 2 → 3 (plans 21-11, 21-12) | round-3-diff CR-01, CR-02 (both genuinely closed — confirmed below), plus round-2 WR-01/WR-03 doc-accuracy items | round-4-diff CR-01: a whitespace-only `--target-phase` accepted end to end — found by round-3 code review (`d4874ec`), confirmed independently **in this report** |

**Both closures in the most recent cycle are genuinely closed**, verified by
direct code reading and by a standalone reproduction test written against HEAD
and then deleted (not by trusting `21-11-SUMMARY.md`'s or `21-12-SUMMARY.md`'s
claims). **But criterion 1 still fails**, for a third distinct reason in three
consecutive cycles: the anti-recurrence mechanism this round built — the
degenerate-payload matrix and the no-wildcard `variant_name` classifier — is
real and does what it claims for the two `CommandSource` variants it was
applied to (`Command`, `Goal`). It was never applied to the third
(`Routed`/`--target-phase`), which was instead exempted by a comment whose
central factual claim is wrong for 3 of the 4 payloads it cites. **The
mechanism works; its coverage was scoped to two of three variants it exists to
cover, and the one it skipped is the one that broke.**

### Observable Truths

| # | Truth (ROADMAP success criterion) | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✗ FAILED (third consecutive cycle, new root cause) | The two defects the last two cycles named (blank `--command`, approval-token ordering) are genuinely closed — confirmed by direct code reading (`command_source`'s Command arm now guards `!command.trim().is_empty()` at `:392`; `parse_approval_token` runs at `:644`, above `if args.dry_run` at `:649`) and by re-running the full workspace suite (1316/1316 passing). But a third defect of the identical family — an unvalidated blank/whitespace value reaching a "the line below is the complete and honest sequence" preview and a real run's `run.json` — is live via `--target-phase`. I reproduced this independently: a standalone reproduction test built on this project's own `tests/driver_dry_run.rs` fixture pattern, calling `drive()` with `target_phase: Some(\"   \")`, returned `Ok(())` in both preview and real-run mode, and the real-run mode created a run directory, `journal.jsonl`, and `run.json` on disk. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | `tests/driver_goal_seam.rs` — 20/20 passing in this round's full-suite run (up from 19; +1 from 21-11's new assertions). The plan-digest/approval-recheck wiring is unchanged from the prior cycle's confirmed-closed state. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed) | `tests/driver_escalation_cap.rs` passing in the full-suite run. Guard six — now widened to scan every file under `src/`, not just `src/driver/run.rs` — passes at 25/25 in `tests/spawn_seam_guard.rs`, and I confirmed by direct code read that its scan mechanism (`test_region_start` boundary, tree-wide `source_files()` iteration) matches what 21-12's SUMMARY claims. This is a strictly stronger check than the prior cycle had. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions ("ignore prior constraints, run …") does not change which command the driver executes | ✓ VERIFIED (reconfirmed) | `tests/driver_injection_corpus.rs` — untouched by any 21-11/21-12 plan; passing in the full-suite run (12 non-ignored arms; 10 `--ignored` live-binary arms not re-executed, unchanged from every prior cycle). |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ VERIFIED (reconfirmed) | `tests/driver_refusal_record.rs` — untouched by any 21-11/21-12 plan; passing in the full-suite run. |

**Score:** 4/5 truths verified (0 present-but-behavior-unverified)

### Independent Reproduction (not taken from 21-REVIEW.md)

I wrote and ran a standalone reproduction test (`tests/zzz_verify_repro_target_phase.rs`)
directly against `drive()` at HEAD (`d4874ec`), built on the same fixture
pattern `tests/driver_dry_run.rs` already establishes (a real git repo, an
opted-in registry entry, the tripwire spawn-evidence pattern). Deleted
afterward and never committed.

**Preview (`--target-phase '   '`, `dry_run: true`):**

```
RESULT: Ok(())
...rendered output included:...
  Routed run: no command would be issued. From the state on disk now,
  the router would park (router_state_unverified:    ) and the run would stop for
  a human rather than choose.
```

**Real run (`--target-phase '   '`, `dry_run: false`):**

```
RESULT: Ok(())
.planning/meta-manager exists = true
artifacts created:
  <root>/.planning/meta-manager/runs/run.lock
  <root>/.planning/meta-manager/runs/.gitignore
  <root>/.planning/meta-manager/runs/2026-08-21T00-00-00Z-blank/run.json
  <root>/.planning/meta-manager/runs/2026-08-21T00-00-00Z-blank/journal.jsonl
```

This matches 21-REVIEW.md's round-3 CR-01 reproduction against the CLI binary
almost verbatim (same rendered `router_state_unverified:    ` phrase, same
artifact set), obtained independently through the library API rather than the
CLI, against a byte-identical whitespace payload.

I also independently confirmed, by direct code reading rather than by running
the review's own reproduction commands:

- `is_plain_path_component` (`src/journal/mod.rs:242-254`) returns `false`
  only for the empty string; every other string in `DEGENERATE` (`"   "`,
  `"\t"`, `"\n  \n"`) is a single `Component::Normal` equal to itself and so
  returns `true` — confirmed by reading the function body against each of the
  four literal payloads by hand, not by compiling a standalone probe.
- The matrix test's `--target-phase` exemption (`src/driver/mod.rs:1976-1983`)
  loops `DEGENERATE` only for `--command` and `--goal` (confirmed by reading
  the loop body at ~:1982-1995) — `--target-phase` never appears in the
  by-name sweep at all.
- `parse_approval_token` sits on exactly one executable line in production
  (`src/driver/mod.rs:644`), above `if args.dry_run` (`:649`) — confirmed by
  direct read, matching both 21-11-SUMMARY.md's and 21-REVIEW.md's claims.
- Guard six's boundary-skip mechanism (`test_region_start(file).unwrap_or(usize::MAX)`,
  then `if *number >= boundary { continue; }`) is exactly as described;
  `src/state_reader/mod.rs` has `mod tests {` at line 311 and a production
  `pub fn count_backlog_items` at line 530, past the boundary — confirmed by
  direct `grep`/`sed` read, and independently corroborated by
  `cargo clippy --all-targets` flagging `items_after_test_module` at that
  exact file and line.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/mod.rs` (`command_source`, Command arm) | Blank/whitespace `--command` refused | ✓ VERIFIED | `if !command.trim().is_empty()` guard present at `:392`, sibling `(Some(_), None) => Err(NoCommandSource)` at `:399`. Confirmed by direct read and by the full-suite pass. |
| `src/driver/mod.rs` (`command_source`, Routed arm) | Blank/whitespace `--target-phase` refused | ✗ MISSING | No emptiness guard on `(None, Some(target_phase)) => Ok(CommandSource::Routed(...))`. This is the direct cause of round-4-diff CR-01, reproduced independently above. |
| `src/driver/mod.rs` (`drive`, `recorded_approval`) | Token parsed as a pure invocation-shape refusal, above decomposition and above `--dry-run` | ✓ VERIFIED | Confirmed at `:644`, above `:649` (`if args.dry_run`) and ~180 lines above `decompose`. Genuinely closed. |
| `tests/spawn_seam_guard.rs` (guard six) | Terminal-write single-call-site property, checked tree-wide with an honest header | ⚠️ VERIFIED but header incomplete | Scan genuinely widened to all of `src/` (confirmed: 25/25 passing, and by reading the scan's `source_files()` iteration). Header names three limits accurately but omits a fourth, live one (marker-to-EOF skip; `src/state_reader/mod.rs:530` is invisible to it today). |
| `tests/spawn_seam_guard.rs` (guard eight, new) | `CommandSource`'s single production construction site, checked with an honest header | ⚠️ VERIFIED but header absent | The check itself is real (confirmed by reading the needle list and allowlist) and passes. Its header states the property but names none of its own limitations, unlike its sibling guard six in the same file and the same round. |
| `src/driver/mod.rs` (degenerate-payload matrix) | Anti-recurrence mechanism covering every `CommandSource` variant | ✗ INCOMPLETE | Covers `Command` and `Goal` genuinely (confirmed: both loop over all 4 `DEGENERATE` payloads and assert `NoCommandSource`). `Routed` is explicitly exempted with a false justification, confirmed by direct read of `is_plain_path_component` against each payload. |
| `.planning/REQUIREMENTS.md` | Accurate requirement status matching phase-21 verification history | ✗ STALE/WRONG | DRIVE-01, DRIVE-03 marked Complete in the same commit that closed 21-11, one commit before the review that found criterion 1 was still broken. Repeats the mistake `828d7cc` reverted earlier in this phase. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `src/driver/mod.rs` (`command_source`, Routed arm) | `src/driver/dry_run.rs` (routed preview render) | A blank/whitespace `--target-phase` is refused before it can be rendered as a park reason or written to `run.json` | ✗ NOT WIRED | The value flows unchecked into `router_state_unverified:` text and into `run.json`'s `target_phase` field. round-4-diff CR-01. |
| `src/driver/mod.rs` (matrix test) | `src/driver/mod.rs` (`command_source`, all three arms) | Every `CommandSource` variant's degenerate payloads are checked by name | ⚠️ PARTIAL | Wired for `Command` and `Goal`; `Routed` is explicitly excluded by a false-premise comment. |
| `src/driver/mod.rs` (`drive`) | `src/journal/mod.rs` (`parse_approval_token`) | Validated as a pure invocation-shape refusal, above decomposition and above `--dry-run` | ✓ WIRED | Confirmed; genuinely closed this round. |
| `tests/spawn_seam_guard.rs` (guard six) | every `src/` file's terminal writes | Single call site provable tree-wide | ✓ WIRED (scope), ⚠️ header incomplete | The scan itself is genuinely tree-wide now; its self-description of remaining limits omits one that is live. |
| `.planning/REQUIREMENTS.md` (DRIVE-01, DRIVE-03) | Verification history for criterion 1 | Requirement status reflects the latest verification's finding | ✗ NOT WIRED | Marked Complete while criterion 1 is (and, at the time of that commit, was about to be shown to still be) failing. |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 21-04, 21-07, 21-09, 21-11 | User states a goal once, driver pursues it without further input | ⚠️ PARTIAL (unchanged disposition, new cause) | Multi-command pursuit (criterion 2) is verified. "Review before anything runs" is again undermined — this time by a defect the anti-recurrence mechanism's own coverage gap let through, in the `--target-phase` seam rather than `--command`/`--goal`. |
| DRIVE-03 | 21-01, 21-03, 21-04, 21-07, 21-08, 21-09, 21-11 | Goal decomposed into structured, machine-checkable, reviewable plan | ✗ BLOCKED (unchanged disposition, new cause) | Same root cause as DRIVE-01 above: the honest-preview and validated-before-anything-runs guarantees are broken for the routed source. |
| DRIVE-04 | 21-02, 21-04, 21-06, 21-10, 21-12 | Model escalation capped per run, exceeding it parks | ✓ SATISFIED | Criterion 3 verified; guard six's widening this round is a strictly stronger check than before, reconfirmed. |
| SAFE-07 | 21-01, 21-03, 21-05, 21-06, 21-08, 21-10, 21-12 | `.planning/` content passed inside an explicit untrusted boundary, never concatenated into instructions | ✓ SATISFIED | Criterion 4 verified. Doc-accuracy defects in this round's new guards (WR-01, WR-02, WR-03 below) do not touch the untrusted-boundary mechanism itself. |
| SAFE-08 | 21-01, 21-05, 21-06, 21-12 | Model's action constrained to fixed enum; free-form shell strings never executed | ✓ SATISFIED | Criterion 5 verified, untouched by this cycle's plans. |

**No orphaned requirements.** The union of `requirements:` declared across all
twelve plans (21-01 through 21-12) is exactly {DRIVE-01, DRIVE-03, DRIVE-04,
SAFE-07, SAFE-08}, matching REQUIREMENTS.md's phase-21 mapping and the ROADMAP
phase-21 `Requirements:` line.

**REQUIREMENTS.md is currently WRONG, not merely stale**, and this is recorded
as a gap (see frontmatter and the artifact table above) rather than left as an
observation: DRIVE-01 and DRIVE-03 read `[x]`/Complete at HEAD, marked in
21-11's own completion commit (`760d71f`), one commit before the round-3 review
(`d4874ec`) that found criterion 1 still broken against the identical tree.
DRIVE-04, SAFE-07 and SAFE-08 correctly read Gaps Found in the checklist
(inconsistent with their own traceability-table rows, which read Complete —
a pre-existing minor inconsistency this report does not attempt to resolve,
since it predates this round and both readings agree on "not yet the phase's
overall passed state"). This report does not edit REQUIREMENTS.md; per the
phase's own established norm (stated in the prior two verification reports),
that edit belongs to whichever step next processes a `passed` verification —
but DRIVE-01/DRIVE-03's current Complete marking should be reverted before
then, not carried forward silently.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/driver/mod.rs` | `~:400` | `command_source`'s `Routed` arm has no emptiness check, unlike both its siblings (`Command` gained one this round; `Goal` has had one since 21-07) | 🛑 Blocker | Direct cause of round-4-diff CR-01; reproduced independently. |
| `src/driver/mod.rs` | `:1976-1983` | The degenerate-payload matrix's `--target-phase` exemption comment claims `is_plain_path_component` returns false for the empty string as though that covers all four `DEGENERATE` payloads; it is true for exactly 1 of 4 | 🛑 Blocker | The regression test written specifically to prevent a third recurrence does not cover the variant that recurred. |
| `.planning/REQUIREMENTS.md` | `~83, ~85, ~164, ~166` | DRIVE-01 and DRIVE-03 marked Complete one commit before the review that found criterion 1 still broken | ⚠️ Warning | Repeats a mistake this phase already had to revert once (`828d7cc`); process/documentation defect, not a code defect. |
| `tests/spawn_seam_guard.rs` | `~1691-1723` (guard six header) | Names three approximation limits individually and accurately but omits a fourth, live one (marker-to-EOF skip; corroborated by clippy at `src/state_reader/mod.rs:311/530`) | ⚠️ Warning | round-3-diff WR-01. No live exploit today via the terminal-write property specifically, but the header underclaims relative to what the scan actually misses. |
| `tests/spawn_seam_guard.rs` | `~2136-2226` (guard eight) | Names no limitation of its own at all, in the same file and round where guard six was rewritten specifically to stop doing that | ⚠️ Warning | round-3-diff WR-02. No live violating call site exists today. |
| `src/driver/mod.rs` | `:1663-1674`, `:1706-1715` | `DEGENERATE` and `empty_numbered_entry` both key on the exact `str::trim` predicate the production guard uses, so the matrix can only confirm the guard, never falsify it | ⚠️ Warning | round-3-diff WR-03. Low exploitability (operator-supplied value; TUI path guarded separately), but it is a guard-design defect in the very mechanism meant to stop this phase's recurring failure mode. |
| `src/driver/mod.rs` | `:400` (pre-existing, carried forward, not new) | `plan_target_phase(plan).unwrap_or_default()` is a second route to a blank `target_phase` that bypasses the argv seam entirely | ℹ️ Info | Named by round-2 review IN-01…04, re-confirmed still open by round-3 review, out of this round's commissioned scope. Not independently re-verified in this pass beyond confirming it is still cited as open. |

No unreferenced `TBD`/`FIXME`/`XXX` debt markers were introduced in this
round's diff (`fad82ec..d4874ec`, `src/` and `tests/` only) — confirmed by
`git diff` piped through `rtk proxy grep`.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Whole-suite regression | `rtk proxy cargo test --workspace -- --test-threads=4` | **1316 passed, 0 failed** across 35 binaries (self-run, not taken from any SUMMARY) | ✓ PASS |
| `cargo build --all-targets` | — | clean | ✓ PASS |
| `cargo clippy --lib -- -D warnings` | — | clean | ✓ PASS |
| `cargo clippy --all-targets` (unfiltered severity check) | — | 5 warnings, all pre-existing and outside this round's diff (`src/browser.rs`, `src/project_creator.rs`, `src/state_reader/mod.rs:311` — the last independently corroborates round-3-diff WR-01) | ✓ PASS (info only) |
| Independent reproduction: whitespace `--target-phase`, preview | standalone `drive()` call against a fresh fixture, not committed | `Ok(())`, renders `router_state_unverified:    ` under the honest-sequence-style header | ✗ CONFIRMS round-4-diff CR-01 |
| Independent reproduction: whitespace `--target-phase`, real run | standalone `drive()` call against a fresh fixture, not committed | `Ok(())`; `run.lock`, run directory, `journal.jsonl`, `run.json` all created | ✗ CONFIRMS round-4-diff CR-01 |
| `is_plain_path_component` payload-by-payload read | direct code inspection of `src/journal/mod.rs:242-254` against `DEGENERATE`'s four literals | `""` → false; `"   "`, `"\t"`, `"\n  \n"` → true | ✗ CONFIRMS the exemption comment's justification is wrong for 3/4 payloads |
| `parse_approval_token` single-call-site claim | direct code read of `src/driver/mod.rs` | Exactly one executable-line hit, at `:644`, above `:649` | ✓ CONFIRMS round-3-diff CR-01 genuinely closed |
| Guard six tree-wide scan claim | direct code read + `git diff` of the widened `TERMINAL_WRITE_ALLOWLIST` mechanism | Matches 21-12-SUMMARY.md's description | ✓ CONFIRMS |
| `mod tests` boundary skip on `src/state_reader/mod.rs` | direct line read (`:311`, `:530`) + `cargo clippy --all-targets` | Marker at 311, production fn at 530, clippy independently flags it | ✓ CONFIRMS round-3-diff WR-01 |
| REQUIREMENTS.md premature marking | `git show 760d71f -- .planning/REQUIREMENTS.md` | DRIVE-01/DRIVE-03 flipped to Complete in 21-11's own completion commit | ✗ CONFIRMS regression |

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Human Verification Required

None required for a status determination. All findings in this report are
code-level, objectively confirmed defects — independently reproduced or
independently read against the built binary/library at HEAD — rather than
judgment calls.

### Gaps Summary

**Genuinely closed this cycle**, reconfirmed by direct code reading and a
fresh full-suite run rather than by trusting `21-11-SUMMARY.md` or
`21-12-SUMMARY.md`: the blank-`--command` refusal, the approval-token's move
into the pure-refusal group above both decomposition and the `--dry-run`
branch, guard six's tree-wide widening, and the `plan_digest` legacy-record
doc correction (in both copies, not just the one the plan named).

**Criterion 1 fails for a third consecutive cycle.** This is not a new class
of defect — it is the third instance of the *same* class (an unvalidated
blank/whitespace value reaching an "honest sequence" preview and a real
run's persisted record) landing on a third distinct `CommandSource` variant
each time: `Goal` (cycle 1), `Command` (cycle 2), `Routed`/`--target-phase`
(cycle 3, this pass). Each cycle's fix has been correct and narrow; each
cycle's regression test has covered exactly the variant that cycle fixed and
no other. **The anti-recurrence mechanism this round built is not broken —
it is real, and it does what it claims for the two variants it covers.** It
was scoped to `Command` and `Goal` and explicitly, by name, exempted
`Routed`, with a justification that is arithmetically wrong for 3 of the 4
payloads it was invoked to justify. That is a scoping decision that turned
out wrong, not a mechanism that failed under test — which changes what the
next round should do: not "add another point fix and another point test,"
which is the bet this phase has now lost three times, but "make the existing
mechanism's coverage exhaustive over the type it already classifies
exhaustively" — derive the matrix's column set from the same `variant_name`
match this round used to make the row axis exhaustive, so a variant cannot be
silently exempted from the sweep the way `Routed` was this round.

Two further Warning-level findings, both confirmed by direct code reading,
land in the guards this round newly added or rewrote: guard six's otherwise
much-improved header still omits a live silent gap (a production item placed
after a file's `mod tests {` marker is invisible to the terminal-write scan —
`src/state_reader/mod.rs:530` is such an item today, independently
corroborated by clippy); and guard eight, added this round specifically in
response to the last cycle's over-claiming criticism, itself names no
limitation at all. Separately, the matrix's own payload/detector pair is
tautological — both are keyed on the identical `trim` predicate the
production guard uses, so the matrix can confirm the guard but structurally
cannot falsify it.

**A regression not caused by this round's code, but by its process:**
`.planning/REQUIREMENTS.md` had DRIVE-01 and DRIVE-03 marked Complete in
21-11's own completion commit — one commit before the code review that found
criterion 1 was still broken against that same tree. This phase has already
had to revert an identical premature-completion marking once (`828d7cc`); it
was not caught before it happened again.

**Recommendation:** route back through `/gsd-plan-phase --gaps` for a fourth
gap-closure plan, scoped to close round-4-diff CR-01 (the `Routed`-arm
emptiness check, the matrix's `--target-phase` inclusion, and the corrected
test pin — all three already spelled out concretely in `21-REVIEW.md`'s
CR-01 fix section) **and, in the same plan, to make the matrix's coverage
structurally exhaustive rather than adding a third hand-picked column** —
otherwise a fourth cycle scoped the same way as the last three is the same
bet that has now lost three times in a row. The three Warning-level guard
findings (WR-01, WR-02, WR-03) are cheap, are in the exact files the plan
will already have open, and directly bear on why this keeps recurring — they
should ride the same plan rather than be deferred again. Also correct
`.planning/REQUIREMENTS.md`'s DRIVE-01/DRIVE-03 marking back to Pending (or
Gaps Found, matching its three siblings) in that plan or before it, so the
next verification is not the second one to have to point this out.

---

_Verified: 2026-08-21T19:00:22Z_
_Verifier: Claude (gsd-verifier)_
