---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 37
subsystem: testing
tags: [rust, argv, proc-cmdline, census, cwe-88, session-detection, untrusted]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "21-36's byte-typed wire harness, its registered `wire_forms()` table, its total two-branch round-trip property with a committed non-vacuity floor, and its strict-UTF-8 parser"
provides:
  - "`every_claude_argv_option_site_under_src_is_adjudicated` — a SOURCE-DERIVED, adjudicated census of every `claude` argv option-literal site under `src/`, with per-file measured counts and three anti-rubber-stamp guards"
  - "`CLAUDE_ARGV_ROUND_TRIPS` — a Producer-coverage invariant: an adjudicated `Producer` with no round-trip test is a RED"
  - "Four measured option spellings: bare short `-r`, attached short `-r<id>`, bare long `--session-id`, fused long `--session-id=`"
  - "A MEASURED rank rule (`--resume` outranks `--session-id`) with its premise ENFORCED by `no_source_line_under_src_requests_a_forked_session`"
  - "`the_executors_own_argv_is_an_argv_this_build_can_read_back` — this build's SECOND producer driven byte-exactly through the real parser"
  - "Two more registered wire forms (`short-split`, `short-attached`), so 21-36's generated property covers the new spellings with no second generator"
  - "The subsumed duplicate control (WR-05) deleted with both survivors OBSERVED red under a planted defect"
affects: [21-verification-pass-14, session-detection, sessions-tab, secure-phase-21]

actuals:
  tokens: 19926
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Source-derived adjudication census with anti-rubber-stamp guards (>=1 Producer, both known producers Producer, every non-Producer reason non-empty, every Excluded reason naming a direction)"
    - "Producer-coverage invariant: a Producer row must be named by a round-trip test, and the named test must exist in the tree"
    - "Deletion-by-observation: plant the defect, observe every survivor red, capture verbatim, restore, then delete"

key-files:
  created: []
  modified:
    - src/session_detector.rs
    - src/ui/screens/detail.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "The census needle is the CLOSED quoted token (`\"--resume\"`, not `\"--resume...`). Both readings were measured; the closed one reproduces the plan's own pre-wave-1 baseline of 18 sites across 4 files EXACTLY (5/8/4/1), the open one gives 25. The plan's prose was disambiguated by measurement rather than by choice."
  - "The anti-self-match needle is 3 heads + 3 tails + a derived fusion character, NOT the plan's literal 5+5 spelling — two of those five heads (`--resume`, `--session-id`) are themselves tokens, so the array would have matched itself, which is the exact property the idiom exists to provide"
  - "`RESUME_OPTION_SHORT_NAME` is spelled PRODUCER-SIDE in `detail.rs` rather than imported from the consumer, preserving 21-36's oracle-independence argument: an encoder derived from the parser it checks can only ever agree with it"
  - "WR-04 is recorded in the BACKLOG with its reproduction and NOT fixed; the file path in the pass-13 report and the plan prose (`src/ui/screens/terminal_switch.rs`) is wrong and was corrected to `src/terminal_switch.rs`"
  - "Zero new dependencies; the `proptest` decline and its measured graph cost (14 packages, lockfile 354 to 368) are recorded durably in `deferred-items.md`"

patterns-established:
  - "A census's per-file COUNT is an assertion only because comments are filtered; that reasoning is written at the constant"
  - "A vocabulary variant kept deliberately unconstructed (`Excluded`) is documented as a contract rather than deleted as dead code, and its reason field is required to name a direction"
  - "Every doc citation of a deleted test is re-pointed in the same commit, in truncated form, so no whole spelling of a nonexistent test survives in the tree"

requirements-completed: [DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08]

coverage:
  - id: D1
    description: "Every `claude` argv option-literal site under `src/` is derived from the tree and adjudicated with a measured per-file count; an unadjudicated site or a moved count is a RED naming the file and the delta"
    requirement: "DRIVE-01"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#every_claude_argv_option_site_under_src_is_adjudicated"
        status: pass
    human_judgment: false
  - id: D2
    description: "The census is not a rubber stamp: at least one Producer row, both known producers are Producer, every non-Producer row carries a non-empty reason, and every Excluded reason names a direction"
    requirement: "DRIVE-03"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#every_claude_argv_option_site_under_src_is_adjudicated"
        status: pass
    human_judgment: false
  - id: D3
    description: "This build's SECOND `claude` argv producer is driven through the same round trip as the first, byte-exactly, in both the non-resuming and the resuming shape"
    requirement: "DRIVE-01"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#the_executors_own_argv_is_an_argv_this_build_can_read_back"
        status: pass
    human_judgment: false
  - id: D4
    description: "The parser reads all four added spellings, and the `--resume` > `--session-id` precedence rule is ENFORCED BY A TEST rather than described by a comment"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape"
        status: pass
      - kind: unit
        ref: "src/session_detector.rs#the_executors_own_argv_is_an_argv_this_build_can_read_back"
        status: pass
    human_judgment: false
  - id: D5
    description: "The rank rule's premise is enforced, not remembered: no non-comment line under `src/` requests a forked session"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#no_source_line_under_src_requests_a_forked_session"
        status: pass
    human_judgment: false
  - id: D6
    description: "The change can only turn `None` into `Some`: every pre-existing parser arm keeps its exact expected value, the single `Untrusted` wrap site survives, and the keep-scanning-past-an-unusable-value behaviour is unchanged"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape"
        status: pass
      - kind: other
        ref: "rtk proxy grep -vE '^\\s*//' src/session_detector.rs | grep -c from_untrusted_source == 1"
        status: pass
    human_judgment: false
  - id: D7
    description: "The two new wire forms are covered by 21-36's generated property and its non-vacuity floor without a second generator"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_round_trip_property_holds_for_every_generated_byte_string"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_generator_reaches_every_named_class_and_both_branches_of_the_property"
        status: pass
    human_judgment: false
  - id: D8
    description: "The subsumed duplicate control (WR-05) is deleted and its safety was OBSERVED, not argued: the fused-branch defect was planted, both survivors went red, the defect was restored, and only then was the duplicate removed"
    verification:
      - kind: other
        ref: "planted-defect run, both verbatim panics quoted in this SUMMARY; restored and green afterwards"
        status: pass
    human_judgment: true
    rationale: "The mechanical half — the name is gone from both files, the count fell by exactly one, all citations re-pointed — is greppable and green. Whether the two survivors genuinely certify everything the deleted control did, rather than merely failing on the one defect that was planted, is a judgment a human must make by reading both."
  - id: D9
    description: "WR-04 (tmux focus-stealing) is recorded in the backlog with its reproduction and is NOT fixed"
    verification:
      - kind: other
        ref: "rtk proxy git diff 916a0c6 HEAD -- src/terminal_switch.rs (EMPTY); deferred-items.md carries the entry"
        status: pass
    human_judgment: false
  - id: D10
    description: "ROADMAP success criterion 4 — the ten `#[ignore]`d arms of tests/driver_injection_corpus.rs"
    verification: []
    human_judgment: true
    rationale: "NOT CLAIMED and no work done against it, for the sixth consecutive round. It needs a human with a live authenticated Claude subscription; no agent can close it. 4/5 is the correct and expected ceiling and is not a failure. `tests/` diff is EMPTY."

duration: 35 min
completed: 2026-08-28
status: complete
---

# Phase 21 Plan 37: The Producer Set Made Measured Summary

**The `claude` argv producer set stops being a set someone remembers and becomes a source-derived, adjudicated census with a Producer-coverage invariant — and the second producer, whose every driver-launched session read back as `None`, is taught to the parser and driven through the same round trip as the first.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-28T04:22Z (approx — baseline measurement)
- **Completed:** 2026-08-28T04:57Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- **The producer set became MEASURED.** `every_claude_argv_option_site_under_src_is_adjudicated` walks `src/`, reports every non-comment line spelling one of this parser's option tokens as a quoted literal, and requires each such file to carry a row in `CLAUDE_ARGV_SITES` with a matching per-file count and a disposition. A file that appears and is not adjudicated is a RED; a count that moves is a RED naming the file and the delta.
- **The census is not a rubber stamp, and that is asserted rather than reviewed.** At least one `Producer` row; both known producers are `Producer`; every non-`Producer` row carries a non-empty reason; every `Excluded` reason must name a direction.
- **The Producer-coverage invariant is what turns the census into a control.** An adjudicated `Producer` with no round-trip test in `CLAUDE_ARGV_ROUND_TRIPS` is a RED — and the census was **committed RED on exactly that**, naming `src/executor/claude.rs`.
- **The second producer's argv is now readable, and driven through the real parser.** `claude -p --session-id <uuid>` returned `None` before this round: every driver-launched session was invisible to the detector that finds it again.
- **Four spellings, each measured at `claude` 2.1.250 by running the CLI, not by copying the plan.** Plus a rank rule derived from the fork option's own help text, with its premise ENFORCED by a test rather than remembered.
- **WR-05 deleted with its safety OBSERVED.** The defect the pair exists to catch was planted, BOTH survivors went red, the defect was restored, and only then was the duplicate removed.
- **Zero new dependencies.** `git diff 916a0c6 HEAD -- Cargo.toml Cargo.lock` is empty.

## Task Commits

1. **Task 1(a): the producer census, committed RED** — `a5c942a` (test)
2. **Task 1(b): the four measured spellings and the measured rank rule** — `b5c0f90` (feat)
3. **Task 1(c): the executor's real argv driven through the real parser** — `83a774d` (test)
4. **Task 2: WR-05 deleted, safety observed** — `d09c645` (test)
5. **Task 3: every pass-13 item dispositioned durably** — `5e93ea3` (docs)

## Files Created/Modified

- `src/session_detector.rs` (+929/−23) — three option constants with their measured-version citations; `SessionIdRank` with the quoted help text, the enforced premise and the disclosed residual; the ranked two-slot exhaustive scan; the dated `# CORRECTED 2026-08-28 (21-37)` block beside the two sentences it corrects; `collect_rs`; the anti-self-match needle assembly; `SiteDisposition`; `CLAUDE_ARGV_SITES`; `CLAUDE_ARGV_ROUND_TRIPS`; `every_claude_argv_option_site_under_src_is_adjudicated`; `no_source_line_under_src_requests_a_forked_session`; `the_executors_own_argv_is_an_argv_this_build_can_read_back`; thirteen new parser arms; two re-pointed doc citations
- `src/ui/screens/detail.rs` (+82/−47) — test module only, **no production line changed**: `RESUME_OPTION_SHORT_NAME`, `encode_short_split`, `encode_short_attached`, two new `wire_forms()` rows, the dated WR-05 note in the round-trip class doc, and the deletion of the subsumed control
- `.planning/phases/.../deferred-items.md` (+335/−0) — five dated sections, appended, never rewritten

## The observed REDs, verbatim

### 1. The census, RED at `a5c942a` on the unadjudicated second producer

```
thread 'session_detector::tests::every_claude_argv_option_site_under_src_is_adjudicated' (4046199) panicked at src/session_detector.rs:1090:13:
src/executor/claude.rs is adjudicated a `claude` argv Producer and NO round-trip test names it in CLAUDE_ARGV_ROUND_TRIPS. The invariant `what this build emits, this build can read back` is stated over THIS BUILD's producers, so a producer with no round trip makes the claim wider than its evidence — which is the defect this census exists to report. Measured: `claude -p --session-id <uuid>` reads back None, so every driver-launched session is invisible to the detector that finds it again.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test session_detector::tests::every_claude_argv_option_site_under_src_is_adjudicated ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1116 filtered out; finished in 0.88s
```

At that commit the count assertions, the unadjudicated-file assertion and all three rubber-stamp guards PASS — the only thing red is the Producer-coverage invariant, which is the gap the round was opened for.

### 2. G2 itself, observed rather than argued

The plan does not require this one; it is here because *"measured before the fix: `claude -p --session-id <uuid>` → `None`"* is a claim, and this round's whole subject is claims wider than their evidence. After `83a774d` the session-id branches were suppressed in the working tree (`SESSION_ID_OPTION_NAME` given a value containing a NUL, which can never occur in an argv element), the new control was run, and the pre-fix behaviour appeared on the **real** driver argv:

```
thread 'session_detector::tests::the_executors_own_argv_is_an_argv_this_build_can_read_back' (4131005) panicked at src/session_detector.rs:1372:9:
assertion `left == right` failed: this build's own executor emitted ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--replay-user-messages", "--session-id", "b6730e21-c3dc-48a0-b650-51b505cc065c", "--setting-sources", "project", "--permission-mode", "dontAsk", "--strict-mcp-config"] and this build could not read the session id back out of it. Measured before 21-37 this returned None for EVERY driver-launched run: ...
  left: None
 right: Some("b6730e21-c3dc-48a0-b650-51b505cc065c")
```

`left: None` on the argv this build actually emits. The suppression was reverted with `git checkout -- src/session_detector.rs`; `grep -c TEMPORARY src/session_detector.rs` is `0` at HEAD and the string appears in no commit.

### 3. WR-05's deletion, PROVEN safe — both survivors red under the planted defect

The parser's fused-long-resume branch was made unreachable in the working tree only (its `strip_prefix` needle replaced by a byte string containing a NUL).

**Survivor 1 — the enumerated control:**

```
thread 'ui::screens::detail::tests::a_session_id_survives_the_round_trip_in_both_wire_forms' (4137799) panicked at src/ui/screens/detail.rs:7884:17:
assertion `left == right` failed: FUSED form: the id "demo\u{200b}" was emitted by resume_terminal_argv("kitty") as ["-e", "claude", "--resume=demo\u{200b}"] and could not be read back out of the wire bytes it produces. This is the shape this TUI itself emits, so a failure here means the TUI cannot re-resume its own session.
  left: None
 right: Some("demo\u{200b}")
```

**Survivor 2 — the generated property:**

```
thread 'ui::screens::detail::tests::the_round_trip_property_holds_for_every_generated_byte_string' (4138426) panicked at src/ui/screens/detail.rs:8312:9:
the round-trip property is FALSE over 4096 generated byte strings in 4 wire forms. 1 violation class(es), one exemplar each:

[refused-a-value-that-must-round-trip] 2655 occurrence(s)
  wire form "fused": the byte string [61, 100, 101, 109, 111, 226, 128, 174] (escaped: "=demo\xe2\x80\xae") is valid UTF-8 and does not trim to empty, so it is in NEITHER refusal class and the parser must return it byte-identically. It returned None. A value this build can put on the wire and cannot read back is a Sessions-tab row that answers `No session ID to resume` with no error anywhere — under-detection, and SILENT.
```

**The duplicate itself was also run under the same planted defect and also went red** (`1 failed`), which is what makes the subsumption a measurement rather than a reading: the defect it exists to catch is caught by both survivors. The defect was then restored; `cargo test --lib ui::screens::detail::tests` returned **62 passed / 0 failed**; and only then was the control deleted.

## The census, MEASURED

### Per-file counts, with every delta explained

| File | Pre-wave-1 baseline | Post-`21-36` (commit `a5c942a`) | Delivered (HEAD) | Disposition |
|---|---|---|---|---|
| `src/executor/claude.rs` | 5 | 5 | 5 | **Producer** |
| `src/session_detector.rs` | 8 | 13 | 23 | Consumer |
| `src/state_reader/git_ops.rs` | 1 | 1 | 1 | NotClaude |
| `src/ui/screens/detail.rs` | 4 | 7 | 8 | **Producer** |
| **Total** | **18 across 4 files** | **26 across 4 files** | **37 across 4 files** | — |

**The pre-wave-1 baseline was REPRODUCED, not inherited.** The plan states 18 across 4 files as 5/8/4/1. Running the needle rule against `git archive 9eeb360` (21-36's base) returns exactly `5 / 8 / 4 / 1`. That reproduction is what settled an ambiguity in the plan's prose — see Deviation 1.

**Every delta named:**

- `src/session_detector.rs` **8 → 13** (`21-36`, +5): the untrimmed-split arm added by G1's fix, plus the four arms of `the_split_forms_successor_is_a_value_by_position_even_when_it_is_option_shaped`.
- `src/ui/screens/detail.rs` **4 → 7** (`21-36`, +3): the three option-shaped entries of `GENERATED_OPTION_PREFIXES` (`b"-r"`, `b"--resume"`, `b"--session-id"`). The fourth pre-existing site moved but did not multiply — the byte retype of the SPLIT-form harness line.
- `src/session_detector.rs` **13 → 23** (this round, +10): three new option constants and seven new parser test arms that spell an option token as a whole quoted literal.
- `src/ui/screens/detail.rs` **7 → 8** (this round, +1): `RESUME_OPTION_SHORT_NAME`.
- `src/executor/claude.rs` and `src/state_reader/git_ops.rs` **unchanged at 5 and 1** — neither file was opened.

### The adjudication, with reasons

| File | Disposition | Reason (abridged; full text at the constant) |
|---|---|---|
| `src/executor/claude.rs` | **Producer** | `build_argv` emits `--session-id <uuid>` on every driver-launched run and `--resume <id>` when resuming. This is the producer the invariant was written about and never asserted over. Round trip: `the_executors_own_argv_is_an_argv_this_build_can_read_back`. |
| `src/ui/screens/detail.rs` | **Producer** | `resume_terminal_argv` fuses the id and emits `--resume=<id>` into a terminal emulator's argv. Round trip: `a_session_id_survives_the_round_trip_in_both_wire_forms`. |
| `src/session_detector.rs` | Consumer | The option constants and the parser arms that pin which shapes carry an id. Builds no argv for anything to execute. |
| `src/state_reader/git_ops.rs` | NotClaude | Its `-r` is `git diff-tree`'s recursive flag, on a `git` argv. **Kept in the table rather than filtered out of the needle**, because a false positive that has to be adjudicated is proof the needle matches more than what it was aimed at. |

**Producer rows: 2 of 4 — not one exclusion in the table.** Both are named by a round-trip test, and both named tests are asserted to exist in the tree (a table naming a test that does not exist certifies nothing — the WR-05 hazard, caught by a walk rather than by a reader).

### What the census does NOT see, with its direction

The needle is a QUOTED OPTION LITERAL, not "an argv destined for `claude`". A producer that assembled its option name from fragments, read it from a config value, or spelled it in a macro is invisible. **Under-detection, and SILENT.** Accepted and DISCLOSED at the census in the source, on the same reasoning `text.rs`'s interpreter census gives for its five interpolation markers. What bounds it is not the walk but the Producer-coverage invariant.

## The dependency measurements, run rather than copied

The plan said to cite `claude` 2.1.250. Rather than assert an unverified number, the CLI was run:

| Measurement | Command | Result |
|---|---|---|
| version | `claude --version` | **2.1.250 (Claude Code)** |
| resume option | `claude --help` | `-r, --resume [value]   Resume a conversation by session ID, or` |
| assigned-id option | `claude --help` | `--session-id <uuid>    Use a specific session ID for the` |
| fork option | `claude --help` | `--fork-session  When resuming, create a new session ID instead of reusing the original (use with --resume or --continue)` |
| complete short-option inventory | `claude --help \| grep -oE '^\s+-[a-zA-Z],' \| sort -u` | **exactly eight: `-c -d -h -n -p -r -v -w`** |

The eight-option inventory is what makes the ATTACHED short form unambiguous: `-r` is the only `r`-initial short option, so an element beginning with those two bytes and longer than them can only be that option carrying a value. That is a dependency behaviour whose measurement **expires**, and the version is cited at each constant.

## The non-negotiables, answered individually

### A — the census is source-derived and adjudicated, with a COUNT

**Four `claude`-argv producer/site files found, 2 of them PRODUCERS, 37 sites total at HEAD.** Derived by walking `src/` with `collect_rs` (the same recursive `read_dir` shape as `crate::text`'s), filtering comment lines, and matching an anti-self-match needle assembled at runtime. Adjudication verdicts in the table above; every non-`Producer` reason is non-empty and asserted so.

### B — the precedence rule is enforced BY A TEST, not a comment

**Two tests enforce it, and they fail in different ways if precedence is inverted.**

1. `session_detector::tests::session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape` asserts
   `parsed(&["claude", "--session-id", "u", "--resume", "old"]) == Some("old")` — the assigned id is
   **further left** and must lose. Inverting the rank makes this arm return `Some("u")` and the test
   red. The companion arm `["claude", "--resume", "old", "--session-id", "u"] == Some("old")` is
   asserted too, deliberately: an implementation that simply returned the first match would pass the
   second and fail the first, so both are needed to pin RANK rather than index.
2. `session_detector::tests::the_executors_own_argv_is_an_argv_this_build_can_read_back` asserts the
   same thing on the argv this build actually emits, with a **non-vacuity guard** proving the
   assigned-id option really does sit to the LEFT of the resume option on that argv
   (`matches!((assigned_at, resume_at), (Some(a), Some(r)) if a < r)`) — otherwise the assertion could
   be satisfied by leftmost-wins and would certify nothing about rank. It additionally asserts
   `assert_ne!(read_back, Some(fresh))`.

**Confirmation it fails if precedence is inverted:** `first_resume.or(first_assigned)` inverted to
`first_assigned.or(first_resume)` makes both of the arms in (1) return `Some("u")` instead of
`Some("old")`, and makes (2) return the fresh uuid, tripping both its `assert_eq!` and its
`assert_ne!`. The rank rule is additionally load-bearing on a premise that is itself a test:
`no_source_line_under_src_requests_a_forked_session`.

### C — `resume_terminal_argv` is byte-identical

`mod tests` in `src/ui/screens/detail.rs` opens at line 5877; `resume_terminal_argv` is at 703 and
`RESUME_OPTION_FUSED_PREFIX` at 585, both far outside it. Every byte of the production half is
unchanged:

```
$ head -5875 <916a0c6:src/ui/screens/detail.rs> > prod_base.rs
$ head -5875 src/ui/screens/detail.rs          > prod_head.rs
$ cmp prod_base.rs prod_head.rs
PRODUCER-CMP-EXIT=0
```

The round's `detail.rs` diff mentions the producer on exactly two lines, and both are **deletions
inside the removed test**: `-    /// producer, [`resume_terminal_argv`], its output is encoded into the`
and `-                let argv = resume_terminal_argv(term, &sid);`. The producer is not un-fused; the
`--` separator was not taken.

### D — zero new dependencies

```
$ rtk proxy git diff --stat 916a0c6 HEAD -- Cargo.toml Cargo.lock
CARGO-END (empty = clean)
```

No output. `cargo add` was never run.

### E — `read_session_id` stays non-validating

Untouched. Filtering the round's `session_detector.rs` diff for `fn read_session_id`, its
`std::fs::read(format!(...))` line and its `session_id_in_cmdline(&cmdline)` call returns nothing on
either side. No first-byte, character-class, length or UUID-shape rule was added anywhere. The only
condition on a VALUE is still non-emptiness after `trim`, tested on a copy.

### F — WR-04 is in the BACKLOG and is NOT fixed

```
$ rtk proxy git diff 916a0c6 HEAD -- src/terminal_switch.rs
(no output)
```

The backlog entry carries the full reproduction (both halves, the `"/dev/pts/31".contains("pts/3")`
demonstration, why it is out of DRIVE-01/03/04 and SAFE-07/08, and the shape a later fix should take
without taking it). **The path in the pass-13 report and in the plan's prose is wrong** — see
Deviation 5.

### G — WR-05's deletion safety was OBSERVED

Both verbatim panics quoted above, one per survivor, plus the duplicate's own red under the same
planted defect. Restored, both green (62 passed / 0 failed in `detail::tests`), then deleted.

```
$ rtk proxy grep -rc 'the_argv_this_build_emits_is_an_argv_this_build_can_read_back' src/ui/screens/detail.rs src/session_detector.rs
src/ui/screens/detail.rs:0
src/session_detector.rs:0
$ rtk proxy grep -c 'a_session_id_survives_the_round_trip_in_both_wire_forms' src/session_detector.rs        -> 3
$ rtk proxy grep -c 'the_round_trip_property_holds_for_every_generated_byte_string' src/session_detector.rs  -> 3
```

Deleted lines in `detail.rs`: **47**, and every one belongs to the removed control — 13 indented
`///` doc lines and 34 body lines, enumerated by
`git diff -- src/ui/screens/detail.rs | grep '^-' | grep -vE '^-\s*///'`. No other doc paragraph in
either file was deleted, shortened or reworded. (On the plan's literal `grep -c '^-///'` criterion,
see Deviation 4.)

### I — wave 1's work is intact, all four properties

| Property | Check | Result |
|---|---|---|
| the `uniform` unrestricted-random-bytes arm exists and is still dispatched | `grep -n 'fn generated_uniform\|// uniform — unrestricted bytes\|let shape = next_random(&mut state) % 5;'` | `7998` / `8020` / `8072` — present, unnarrowed, still dispatched at shape 0 out of 5 |
| the non-vacuity floor `#[test]` exists with per-class assertions | `grep -c 'fn the_generator_reaches_every_named_class_and_both_branches_of_the_property'` = **1**; `grep -c 'below the floor of'` = **6** | six per-class floors, each its own assertion, plus the three `>= 1` branch counters |
| the oracle is computed in-test from `std`, and `session_detector` exports no shared refusal predicate | `grep -n 'let r1 = decoded.is_err();'` → `8238`; `let r2 = decoded.map(...)` → `8239`. Export surface: `pub struct ClaudeSession`, `pub fn detect_sessions`, `pub(crate) fn session_id_in_cmdline` | no `is_refused`, nothing for the test to import that could make it agree by construction |
| the parser refuses ill-formed input; no `from_utf8_lossy` in `session_id_in_cmdline` | `grep -vE '^\s*//' src/session_detector.rs \| grep -n from_utf8_lossy` → **one line only**, `get_claude_pids`'s `String::from_utf8_lossy(&output.stdout)` (pre-existing, out of scope, documented by 21-36) | strict `std::str::from_utf8` with the scan continuing, unchanged |

Additionally: `grep -vE '^\s*//' src/session_detector.rs | grep -c from_untrusted_source` is still
exactly **1** — one wrap site, still at the innermost point of the pair, now inside the
`if slot.is_none()` guard.

## The gate, measured

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | exit **0** |
| `rtk proxy cargo test --lib` | **1118 passed; 0 failed; 0 ignored** (21-36 delivered **1116**; +3 new, −1 deleted) |
| `rtk proxy cargo test --no-fail-fast` (workspace) | **1432 passed; 0 failed; 13 ignored** across **35** binaries on a clean run (21-36 delivered **1430**; +2 net) |
| `rtk proxy cargo clippy -- -D warnings` | exit **0** |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | fails with **exactly four** pre-existing lints: three `assert_eq!`-with-a-literal-bool in `src/browser.rs` (155, 156, 157) and one `cmp_owned` in `src/project_creator.rs:146` |
| `git diff --stat 916a0c6 HEAD -- tests/ .planning/REQUIREMENTS.md .planning/ROADMAP.md .planning/STATE.md Cargo.toml Cargo.lock src/executor/claude.rs src/terminal_switch.rs` | **EMPTY** |
| placeholder markers in added `src/` lines | **0** |
| `test-threads=1` added anywhere in `src/` or `tests/` | **0** |
| `grep -rc 'fork-session' src/` | **0 for every file** — the guard's needle is assembled, and the one doc line that spelled it whole was reworded |
| `grep -c '2.1.250' src/session_detector.rs` | **17** (criterion: at least 2) |
| files changed vs plan base | exactly the three in `files_modified` |

**Every `--lib` delta attributed.** +3 = `every_claude_argv_option_site_under_src_is_adjudicated`,
`no_source_line_under_src_requests_a_forked_session`,
`the_executors_own_argv_is_an_argv_this_build_can_read_back`. −1 =
`the_argv_this_build_emits_is_an_argv_this_build_can_read_back`, named in Task 2. Thirteen new arms
were added to the existing `session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape`, which is
why the count does not move by more. **No pre-existing expected value changed** — verified by
inspection of the diff (the pre-existing arms appear as context, not as `+`/`-` pairs) and by the test
being green.

### Every removed line, both source files

`src/session_detector.rs` — **23**: seven `///` lines (the two re-pointed citations, replaced in the
same commit) and sixteen executable / inline-comment lines of the scan restructure. The
`// The ONE place a session id is WRAPPED` comment and its `from_untrusted_source` line reappear
inside the new `if slot.is_none()` guard, which is why the wrap count is still 1.

`src/ui/screens/detail.rs` — **47**: 13 indented doc lines and 34 body lines, all belonging to the
deleted control. Zero other deletions; the one apparent deletion in the `wire_forms()` hunk is the
function's own signature line, re-added unchanged.

`deferred-items.md` — **0**. `git diff … | grep -c '^-[^-]'` is `0`; the file is appended to.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The plan's five-head needle spelling defeats its own anti-self-match property**

- **Found during:** Task 1(a)
- **Issue:** The plan specifies heads `--resum`, `--resume`, `-`, `--session-i`, `--session-id` paired with tails `e`, `=`, `r`, `d`, `=`, and gives the reason: *"a middle split would leave a fragment sitting in the array that is itself the token being looked for."* Two of those five heads — `--resume` and `--session-id` — **are** tokens being looked for. Spelled that way, the constant's own line is a site and the census reports itself.
- **Fix:** Three heads (`--resum`, `-`, `--session-i`), three tails (`e`, `r`, `d`), and the two fused spellings DERIVED by appending a separate `OPTION_NEEDLE_FUSION` constant to the assembled bare token. Five tokens, still split on the LAST character, and now genuinely anti-self-matching.
- **Verification:** measured before and after the census code landed — per-file counts identical (`5/13/1/7`), so the census code adds zero sites of its own. The reason is written at the constant, naming the rejected spelling explicitly.
- **Committed in:** `a5c942a`

**2. [Rule 3 - Blocking] The plan's needle definition was ambiguous; the ambiguity was settled by reproducing the plan's own baseline**

- **Found during:** Task 1(a)
- **Issue:** *"spelling one of this parser's option tokens as a quoted literal"* admits two readings: a quoted literal that IS the token (`"--resume"`), or one that merely BEGINS with it (`"--resume=abc"` would count). The two give very different numbers.
- **Fix:** Both were measured against `git archive 9eeb360` (21-36's base), where the plan's own pre-wave-1 baseline of **18 across 4 files as 5/8/4/1** is stated. The closed reading reproduces it **exactly**; the open reading gives 25 (5/14/1/5). The closed reading was therefore adopted as the one the plan measured. The reasoning is recorded at `assembled_option_needles`: a needle carrying both quotes matches an option SPELLING and not a fixture VALUE, which is what keeps the per-file numbers from moving every time a test arm is added.
- **Committed in:** `a5c942a`

**3. [Rule 2 - Missing Critical] `SiteDisposition::Excluded` is never constructed, which would have made the clippy gate five lints instead of four**

- **Found during:** Task 1(a)
- **Issue:** No site in this tree needs `Excluded`, so rustc's `dead_code` fired on the variant. Under `cargo clippy --all-targets -- -D warnings` that is a **fifth** lint, and the gate requires exactly four pre-existing ones. Deleting the variant would have been the easy fix and the wrong one: the vocabulary is the census's contract, and an adjudicator who meets a genuine exclusion and has no word for it files the site under a wrong one.
- **Fix:** `#[allow(dead_code)]` on the variant with the reasoning written at it, **plus** a new assertion that any `Excluded` row's reason must name a DIRECTION (`under-detection` / `mis-detection`) — so the variant is not merely preserved but constrained.
- **Verification:** `cargo clippy --all-targets -- -D warnings` reports exactly the four pre-existing lints.
- **Committed in:** `a5c942a`

**4. [Rule 1 - Bug] Task 2's `grep -c '^-///'` criterion measures column-0 doc deletions only**

- **Found during:** Task 2
- **Issue:** The criterion says the count *"counts ONLY the deleted control's own doc block"*. The deleted control's doc block is inside `mod tests` and therefore **indented**, so it appears in the diff as `-    ///` and the criterion's anchored pattern does not match it. Measured, `git diff -- src/session_detector.rs src/ui/screens/detail.rs | grep -c '^-///'` is **7**, and all seven are in `session_detector.rs` — the two re-pointed citations, which Task 2 *requires*.
- **Fix:** No code change; measured precisely instead, on the pattern that answers the question the criterion was asking. `git diff -- src/ui/screens/detail.rs | grep -cE '^-\s*///'` is **13**, and every one of the 47 deleted lines in that file belongs to the removed control — enumerated in full above so nothing hides in an aggregate.
- **Committed in:** n/a — recorded here rather than silently satisfied

**5. [Rule 1 - Bug] WR-04's file path is wrong in both the pass-13 report and the plan**

- **Found during:** Task 3
- **Issue:** The plan's prohibition, its `read_first` and its acceptance criteria all spell the defect site as `src/ui/screens/terminal_switch.rs:57`. **There is no such file.** `terminal_switch` is a top-level module declared in `src/lib.rs`; the defect is at **`src/terminal_switch.rs:57`**. A backlog entry pointing at a path that does not exist is a backlog entry nobody can act on — which is the exact failure mode a backlog entry exists to prevent.
- **Fix:** The defect was located and read at the real path; the backlog entry records the correct path **and** notes the discrepancy so a later round greps for something that exists. The plan's `grep -c 'terminal_switch.rs:57'` criterion is satisfied either way (it matches the suffix). The verification item `git diff -- src/ui/screens/terminal_switch.rs` was additionally run against the REAL path — empty, so WR-04 is genuinely untouched.
- **Committed in:** `5e93ea3`

**6. [Rule 1 - Bug] Task 3's `grep -c '^-'` = 0 criterion is unsatisfiable for any non-empty diff**

- **Found during:** Task 3
- **Issue:** `git diff -- <file>` always emits the `--- a/<file>` header, which starts with `-`. The criterion can never be `0` for a file that changed at all.
- **Fix:** Measured on the pattern that answers the intent — `grep -c '^-[^-]'`, the same form round 12's own record uses — which is **0**. The raw `grep -c '^-'` is **1**, the header. The file is append-only: `git diff --numstat` reports `335 0`.
- **Committed in:** n/a — recorded here rather than silently satisfied

**7. [Rule 3 - Blocking] One doc line spelled the fork option whole, breaking its own guard's acceptance criterion**

- **Found during:** Task 1(b)
- **Issue:** The `SessionIdRank` residual paragraph wrote `claude --resume X --fork-session`, so `grep -rc 'fork-session' src/` reported `1` instead of `0`. The guard itself was unaffected (it filters comments), but the criterion measures the whole file and the criterion is right to: a spelling in prose today is a spelling in code tomorrow.
- **Fix:** Reworded to name the flag as `--fork-` + `session` with the reason stated inline. `grep -rc 'fork-session' src/` is now `0` for every file.
- **Committed in:** `b5c0f90`

### Sequencing note, not a deviation

**Commit `b5c0f90` is not fully green, by the plan's own design.** Its heading says *"make the census green"*, but the census's Producer-coverage RED can only close once the executor round trip exists, which the plan places in the next commit. `b5c0f90` therefore reports `1117 passed; 1 failed` — the census, still red on exactly the gap it was committed red for — and `83a774d` closes it. This is stated rather than smoothed over, because a reader bisecting the round will meet that red.

---

**Total deviations:** 7 (2 bugs in the implementation's path, 3 defects in the plan's own acceptance
criteria recorded as measurements, 1 missing-critical, 1 blocking).
**Impact on plan:** No scope creep and no narrowed claim. Deviation 1 is the only one that changes
what was built rather than correcting a spec slip, and it strengthens the census in exactly the
dimension the round is about — a census that reports itself is a census whose numbers mean nothing.
Deviations 4, 5 and 6 are defects in the plan's criteria and are recorded as measurements rather than
quietly satisfied.

## Issues Encountered

**The `tests/driver_reattach.rs` flake fired, and it is the documented pre-existing flake — not a
regression.** Recorded explicitly, per this phase's standing instruction, rather than absorbed
silently.

Measured this session, all at HEAD, same binary, **no code change between runs**:

| Run | Result |
|---|---|
| workspace run 1 (`--no-fail-fast`) | FAILED — 1 passed, **2 failed** |
| `--test driver_reattach` run 1 | FAILED — 1 passed, **2 failed** |
| `--test driver_reattach` run 2 | FAILED — 2 passed, **1 failed** |
| `--test driver_reattach` run 3 | **ok — 3 passed, 0 failed** |
| workspace run 2 (`--no-fail-fast`) | 1431 passed, **1 failed**, 13 ignored |
| workspace run 3 (`--no-fail-fast`) | **1432 passed, 0 failed, 13 ignored** |

The failure **count** varies run to run at an identical commit (2 / 2 / 1 / 0), and two runs were
fully green. That non-determinism is the signature. The failing assertions are also plainly unrelated
to anything this plan touched:

```
a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step
  assertion `left == right` failed: exactly one project has a run to observe
    left: 0
   right: 1

a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired
  the run record is on disk: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

Both are about run-journal records on disk. Neither `session_id_in_cmdline`, `build_argv`,
`nul_join_cmdline` nor any generated byte string is reachable from either. **`--test-threads=1` was
NOT reinstated** — that is a settled decision, and round 11's 4-of-6 measurement on an idle machine
refutes concurrency as a necessary cause.

**Measurement note on the untracked-files verification item.** The plan requires
`git status --porcelain | grep -cE '^\?\? (\.gsd/|\.planning/milestone\.lock)'` to be `2`. In this
worktree it is **0**: neither `.gsd/` nor `.planning/milestone.lock` exists here — they are untracked
files in the main checkout, which a `git worktree` does not carry. The property the item protects —
that neither is staged or committed — holds: the working tree is clean and the round's diff names
exactly the three declared files.

## Known Stubs

None. No hardcoded empty value, placeholder string or unwired component was introduced; the
placeholder-marker grep over added `src/` lines returns 0. `SiteDisposition::Excluded` is
deliberately unconstructed and is **not** a stub — it is a vocabulary entry with a documented
contract and an asserted constraint on its reason field (Deviation 3).

## Threat Flags

None. The three files' diff introduces no new network endpoint, auth path, file-access pattern or
schema change. The one new *behaviour* at a trust boundary — accepting `--session-id` as an id source
from `/proc/<pid>/cmdline` — is `T-21-37-02` in the plan's own register, disposition `mitigate`, and
is mitigated as specified: bounded by the measured rank rule and by
`no_source_line_under_src_requests_a_forked_session`, with its residual (an externally-launched forked
session) disclosed at `SessionIdRank` and in `deferred-items.md`.

## What is honestly NOT claimed

- **The census sees quoted literals, not intent.** A producer that assembles its option name from
  fragments, reads it from config, or spells it in a macro is invisible. Under-detection, and silent.
  Accepted, disclosed in the source, and bounded by the Producer-coverage invariant rather than by the
  needle.
- **The rank rule's premise is `claude`'s.** An externally-launched forked resume has an id this
  parser would misreport. Mis-detection, bounded to a population this build never creates, and guarded
  by a test that fires if this build starts creating it.
- **The spellings are a dependency behaviour measured at 2.1.250.** Six spellings are read; that is
  not a claim that six is all there are, and the measurement expires with the CLI.
- **DRIVE-01, DRIVE-03 and SAFE-08 remain flagged unverified as WHOLE requirements.** This round's
  diff reaches the session-detection loop and the census; goal decomposition, the
  machine-checkable-plan half of DRIVE-03 and `parse_action`'s fixed command enum are all untouched.
  All three are written into `deferred-items.md` as flagged assumptions with the reason each could not
  be resolved, and the no-silent-drop equality is stated there: **7 probe-surfaced == 4 authored as
  explicit truths in 21-36 + 3 flagged here.**
- **WR-04 is a real bug that is still there.** It is in the backlog with its reproduction, not fixed,
  by decision.
- **ROADMAP success criterion 4 is re-surfaced with no work claimed against it, for the sixth
  consecutive round.** The ten `#[ignore]`d arms of `tests/driver_injection_corpus.rs` need a human
  with a live authenticated Claude subscription. **4/5 is the correct and expected ceiling and is not
  a failure.** That file was not edited; `tests/` has an empty diff. The criterion-4 paragraph in
  `deferred-items.md` was verified byte-identical against `ROADMAP.md:450`:
  `diff` empty, `cmp` exit `0`.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Both gap-closure plans of round 13 are delivered.** G1 (value axis) and G3 (encoding axis) by
  `21-36`; G2 (producer axis) plus WR-05 and the full pass-13 disposition by this plan.
- **What a pass-14 verifier should check first, in the spirit of pass 13's own closing note:** the
  census's needle is a curated pattern, and this round's argument is that its FALSE POSITIVE
  (`git_ops.rs`) proves it matches more than it was aimed at. That is evidence, not proof. The
  verbatim-extraction method is cheap here: plant a new option-literal site in a fresh file under
  `src/` and confirm the census names it.
- **Carried forward, not closed:** ROADMAP criterion 4 (needs a human); WR-04 (backlog, with its
  reproduction and its corrected path); IN-01 (backlog, unreachable today, one flag change from live);
  IN-02/IN-03/IN-05/IN-06 (accept, dispositioned); the three unclassified probe rows, which need a
  `21-SPEC.md` rather than more tests; and the `driver_reattach` flake.

## Self-Check: PASSED

| Check | Result |
|---|---|
| `21-37-SUMMARY.md` exists on disk | FOUND |
| all five task commits present in `git log 916a0c6..HEAD` | FOUND — `a5c942a`, `b5c0f90`, `83a774d`, `d09c645`, `5e93ea3` |
| working tree clean before the SUMMARY commit | clean (`git status --porcelain` empty) |
| `.planning/STATE.md` / `ROADMAP.md` / `REQUIREMENTS.md` diff vs base | EMPTY (orchestrator owns those writes) |
| commit-a (`a5c942a`) carries the census and is RED on the Producer gap | verbatim panic quoted above; `1116 passed; 1 failed` at that commit |
| the two temporary probes left no trace | `grep -c TEMPORARY src/session_detector.rs` → `0`; both reverted with a targeted `git checkout -- <file>` |
| all plan `<verification>` commands re-run | see "The gate, measured" |
| all task `<acceptance_criteria>` re-run | all pass, except the three recorded as plan defects in Deviations 4, 5 and 6 |

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-28*
