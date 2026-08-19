---
phase: 20-deterministic-decision-router-run-bounds
plan: "05"
subsystem: driver
status: complete
tags: [driver, rate-limit, quota, ctrl-07, drive-06, park, fixture, tolerant-parsing]
requires:
  - src/executor/stream_json::StreamMessage::RateLimitEvent
  - src/executor/ExecutionEvent::Message
  - src/journal::JournalEvent::Parked
  - src/driver/run::Terminal
  - src/driver/run::terminal_label
provides:
  - src/driver/rate_limit::classify
  - src/driver/rate_limit::window
  - src/driver/rate_limit::reset_time
  - src/driver/rate_limit::park_detail
  - src/driver/rate_limit::terminal_reason_names_a_rate_limit
  - src/driver/rate_limit::QuotaWindow
  - src/driver/rate_limit::QuotaVerdict
  - src/driver/rate_limit::QuotaReason
  - src/driver/rate_limit::REASON_QUOTA_REJECTED
  - "src/driver/run::Terminal::QuotaParked"
  - tests/fixtures/transcripts/09-rate-limit-rejected.ndjson
affects:
  - src/driver/rate_limit.rs
  - src/driver/run.rs
  - src/driver/mod.rs
  - src/journal/mod.rs
  - tests/driver_rate_limit.rs
  - tests/fixtures/transcripts/09-rate-limit-rejected.ndjson
  - tests/fixtures/transcripts/README.md
tech-stack:
  added: []
  patterns:
    - "pure classifier with the clock handed in by the caller (D-11), so a fixed fixture timestamp is judged against a chosen `now` and no assertion is a time bomb"
    - "tolerant `&str` matching with an explicit fallback arm carrying the observed value verbatim"
    - "verbatim in the type, bounded and control-character-free at the journal-rendering boundary"
    - "source-scanning signature assertion with a live control arm on a synthetic widened declaration"
    - "synthesised transcript fixture labelled in the directory README, with the inventory enforced by a test in both directions"
key-files:
  created:
    - src/driver/rate_limit.rs
    - tests/driver_rate_limit.rs
    - tests/fixtures/transcripts/09-rate-limit-rejected.ndjson
  modified:
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/journal/mod.rs
    - tests/fixtures/transcripts/README.md
decisions:
  - "the quota reason is a FOURTH sibling taxonomy (`quota_`) in rate_limit.rs, not a fifth BoundsReason arm"
  - "the second detector matches `contains(\"rate_limit\")`, not the research document's `starts_with(\"api_error\")`"
  - "the second detector reports the window as unknown even when an `allowed` payload was retained"
  - "the quota park applies to single-command mode as well as routed mode"
  - "classify/window/reset_time take the retained event whole; all wire field names live in rate_limit.rs"
metrics:
  duration: ~2h
  completed: 2026-08-19
actuals:
  tokens: 25000
  tasks: 3
  commits: 5
---

# Phase 20 Plan 05: The Subscription Quota Park Summary

A run that hits a Claude subscription quota now **stops**, names which window
blocked it and when it resets, and does not retry — with the signal read in the
driver's drain loop where it already arrives, and with
`derive_run_outcome_from_envelopes`'s signature proven byte-identical by a
scanner that carries its own control arm.

## What Was Built

**`src/driver/rate_limit.rs` — the pure classifier.** No I/O, no clock: the
caller retains the payload off the stream and hands in the instant to judge a
reset time against, in the same register as `bounds` and `router` (D-11). Three
public questions, each answerable about a malformed payload without panicking:

- **`window`** maps `rateLimitType` by string match with an explicit fallback.
  `five_hour` → `FiveHour`; all four seven-day members (`seven_day`,
  `seven_day_opus`, `seven_day_sonnet`, `seven_day_overage_included`) → `SevenDay`;
  anything else → `Unknown(Some(observed))` carrying the string verbatim. An
  absent, non-object or type-less payload → `Unknown(None)`. The `Option` inside
  `Unknown` is load-bearing: a build that flattened both cases into an empty
  string could not tell *"the CLI named a window we have not seen"* — an ordinary
  forward-compat event on a CLI that shipped three new members in one version
  line — from *"there was no window to name"*.
- **`reset_time`** reads `resetsAt` through `as_i64`, so a float, a string, an
  object and an absent field all yield no reset time rather than a rounded or
  parsed one. It then constructs with `DateTime::from_timestamp`, which
  **refuses** rather than wrapping, and only then applies the ±30-day sanity
  bound. The construct-then-bound order keeps both arms reachable and tested.
- **`classify`** parks on `status: "rejected"` and on nothing else.

**`terminal_reason_names_a_rate_limit` — assumption A2's second detector.** A
failure envelope whose own `terminal_reason` names a rate-limit condition parks
under the same reason. It reads a field the driver already holds, so **nothing
about the outcome derivation changes.**

**The observation, in the arm that already receives it.** `run.rs`'s drain-loop
event arm now retains the latest `rate_limit_event` **uninspected** — every
question about it is `rate_limit`'s. The turn-boundary computation, the journal
write, the error-kind-only log line and the stream-close break are untouched, and
no second consumer of the stream was added.

**`Terminal::QuotaParked`, a fourth arm.** It writes through the same
`JournalEvent::Parked` carrier every other park uses, so `terminal_label` picks
it up through the `parked:` prefix with **no change**, and `run.json` records
`parked:quota_rejected` for a separate process to read.

**The park stops the run.** No retry, no backoff, no sleep-until-reset, and
nothing scheduled against the reset time. A test measures the interval between
the `parked` and `run_ended` journal records against a fixture whose reset is two
days out, so a reintroduced backoff cannot pass unnoticed.

**`09-rate-limit-rejected.ndjson`, labelled as synthesised.** The one status no
capture exists for. Field names and enum values are verbatim from the 2.1.235
binary's string table; every envelope around the event follows the real captures'
shape. The README's opening contract — *"Every `.ndjson` file in this directory is
a **real capture**"* — became false the moment this file landed, so it was
restated rather than left to be discovered, and a test now enforces the inventory
against disk in both directions.

## Key Decisions

**The quota reason is a fourth sibling taxonomy, not a fifth `BoundsReason`
arm.** The plan offered "the bounds or router sibling enum"; both are wrong and
both are outside this plan's declared file list (`router.rs` belongs to 20-04,
running in a parallel worktree). `bounds.rs`'s own module doc states that CTRL-06
has **four** detectors and that they are the only stopping condition an
unattended run has — a fifth arm would falsify that sentence. The distinction is
real rather than bookkeeping: the four bounds are facts about *this run's* budget,
while a quota rejection is a fact about a budget **shared with every other Claude
surface the user has**, which is exactly why it must not retry. The plan's own
artifacts list already named `quota_rejected` as a new constant alongside
`rate_limit.rs`, so this is the reading the plan converged on.

**The park applies to single-command mode too.** Single-command mode remains
byte-for-byte the Phase 17 run for every run that is not rate-limited (proven —
`single_command_mode_is_untouched_by_the_iteration_loop` still passes, and this
plan's own allowed-event test asserts `succeeded_no_changes` with no `parked:`
prefix). But a Phase 17 run that hit a quota reported a bare `failed` with no
reason at all, which is precisely the unclassified ending DRIVE-06 exists to
remove.

**The classifier takes `now` rather than reading a clock, and that is what keeps
the fixtures honest.** The committed captures carry fixed `resetsAt` values from
July 2026. A sanity bound measured against the wall clock would make every
fixture assertion a test that passes this month and fails the next — the fixture
would silently stop being inside the bound and the assertion would start proving
the calendar. Tests judge each fixture against a `now` derived from that
fixture's own value.

## Deviations from Plan

### Corrected plan and research premises

**1. [Rule 1 - Bug] The research document's second-detector predicate would have
reported every API error as a quota exhaustion.** Assumption A2's mitigation
proposes `terminal_reason.starts_with("api_error")`. That fires on
`api_error_overloaded`, on a plain `api_error`, and on whatever `api_error_*`
value ships next — none of which is a quota condition. The consequence is not
cosmetic: the run would name the wrong cause and tell the user to wait out a
five-hour or seven-day window for a fault a retry would have cleared in seconds,
which is exactly the misinformation CTRL-07's transparency prohibition is about.
The predicate is `contains("rate_limit")`, with
`an_ordinary_api_error_is_not_reported_as_a_quota_exhaustion` pinning seven
non-quota terminal reasons — including `budget_exhausted`, which is the
`--max-budget-usd` post-turn breaker and a different constraint entirely (D-16).
**Commit:** `b362187`.

**2. [Rule 2 - Missing critical functionality] The second detector reports the
window as `unknown` even when a payload WAS retained.** The plan says "with the
window reported as unknown when no payload was retained", which implies borrowing
one when there is a payload. The only payload it could borrow from is one that
classified `Allowed` — by definition an event that did **not** describe this
refusal. Reporting its window would be a guess wearing a fact's clothes, and
CONTEXT.md's standing commitment is to report unknown rather than guess. The
end-to-end test for this path asserts the `unknown` explicitly.

**3. [Rule 2 - Missing critical functionality] The observed window string is
bounded and control-character-stripped where it reaches the journal.** T-20-27
requires that no payload body reach the journal, and the journal lands in
`.planning/`, a directory users commit, as one NDJSON record per line. A
`rateLimitType` carrying a newline is a record that reads as two, and an
unbounded one is a record no reader wants whole. The **type** keeps the value
verbatim, which is what "never mapped, never dropped" requires; the bound
(`MAX_OBSERVED_WINDOW_CHARS = 64`) and the control-character filter apply only at
`QuotaWindow::detail`, the journal-facing rendering.
`an_observed_window_string_reaching_the_journal_is_bounded_and_line_safe` drives
it with an 8 KB hostile string containing `\n` and `\r`. **Commit:** `b362187`.

**4. [Rule 2 - Missing critical functionality] `JournalEvent::Parked.reason`'s
own doc said "Three sanctioned taxonomies" while a fourth was about to ride the
same field.** That doc is 20-02's, and its whole point is that naming only some
of the taxonomies "would be the same quiet lie this record exists to prevent" —
so landing a fourth without amending it would have reproduced the defect the
paragraph was written against. `src/journal/mod.rs` is outside this plan's
declared file list; the change is doc-only (the table gains a `quota_` row and
two counts change), the file is touched by no other plan in this wave, and the
alternative was to ship a statement this plan made false. **Commit:** `9148280`.

### Recorded design notes

**5. The park detail lands on a preceding `diagnostic` record, not on the
`parked` record itself.** That is the pre-existing carrier convention
(`record_terminal`'s own doc: `needs` names the actor that would unpark the run
and must not be overloaded with an observed value), so the quota park reuses it
rather than inventing a field. `Terminal::detail()` was added as an exhaustive,
wildcard-free accessor so an arm added later is a decision at that site rather
than a silent fall-through to "no detail".

**6. Fixture 09 declares `claude_code_version: "2.1.235"`, not `2.1.220`.** Its
enum values come from the 2.1.235 inventory, so declaring 2.1.220 would have been
a small lie in a file whose entire value is that its contents match the wire. It
sits above `TESTED_MAXIMUM_CLAUDE_VERSION`, so it also exercises the capability
gate's warn-and-proceed arm.

**7. Task 2's tests were written against a reverted `run.rs` to obtain a genuine
RED.** The drain-loop change was drafted first, then set aside
(`git checkout -- src/driver/run.rs`, with the work saved outside the tree) so
the eight end-to-end tests could be observed failing before the implementation
landed. Five failed RED; the three that passed are the source-scanner, its
control arm, and the allowed-event negative control, all three of which are
correct to pass without the feature. The no-sleep test **also** passed vacuously
in that first RED run — without a quota detector the run still parks promptly on
`bounds_command_repeat` — so a non-vacuity assertion pinning the park's reason
was added before the RED was committed, taking the RED count from four to five.

## Known Stubs

None. No stub, placeholder, TODO, skipped test or unrun `<verify>` was
introduced by this plan.

Two pre-existing stubs remain untouched and are unaffected:
`router::Decision::GoalMet` and `RouterReason::DependencyUnsatisfied` still have
no producer (inherited from 20-01, and `router.rs` belongs to 20-04 in a parallel
worktree).

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean |
| `rtk proxy cargo test --no-fail-fast` | **28 suites, 0 failures.** 907 lib tests |
| `cargo clippy -- -D warnings` (documented gate) | clean |
| `cargo clippy --all-targets -- -D warnings` | exactly the **5** pre-existing lints recorded in `TESTING.md`, at the same locations (`browser.rs:131-133`, `project_creator.rs:146`, `state_reader/mod.rs`) |
| `rtk proxy cargo test --lib rate_limit` | 19 passed |
| `rtk proxy cargo test --test driver_rate_limit` | 13 passed |
| `rtk proxy cargo test --test driver_iteration_loop` | 5 passed (unchanged) |

Every `rtk proxy` is deliberate: `rtk` strips cargo's `warning:` and
`test result:` lines, so a grep of those against bare `cargo` succeeds vacuously.

**The signature assertion is proven live, not merely green.**
`the_one_outcome_entry_point_still_takes_exactly_the_full_result_envelopes`
compares the whitespace-normalised declaration against a literal; its control arm
`the_signature_scanner_fires_on_a_synthetic_widened_declaration` runs the same
extractor over a synthetic declaration carrying an extra
`rate_limit: Option<&serde_json::Value>` parameter and asserts three things — that
the matcher still **finds** a declaration, that it **captures** the added
parameter, and that it **rejects** the result. A third arm asserts the extractor
returns `None` where the declaration is absent, so a rename fails loudly rather
than passing silently.

**`driver_reattach` passed 3/3 on every run of this session's final suites.** The
orchestrator's brief records it as failing 2/3 pre-existing and bisected to a
docs-only commit; it was not touched here, and it is green in this tree. One
`driver_lock::the_lock_is_released_when_the_holding_process_dies` failure was
observed once mid-session and passed on immediate re-run and on every subsequent
full-suite run — the same one-off recorded in commit `641a9c8`. Nothing in this
plan is on the lock path.

**Isolation verified.** `git diff --numstat` over all five commits touches seven
files; **none** is `STATE.md`, `ROADMAP.md`, `WINDOWS.md`, `src/ui/*`,
`src/driver/router.rs`, `src/envelope/mod.rs`, `tests/driver_router_table.rs` or
`tests/driver_router_conformance.rs`. `src/driver/router.rs` is read from in one
test assertion (its `REASON_*` constants, for the taxonomy-collision check) and
is not modified.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema change at
a trust boundary was introduced beyond what the plan's `<threat_model>` already
registered. The one addition to the record's content — the observed window string
— is covered by T-20-27's mitigation and is bounded and line-safe by deviation 3.

## Self-Check: PASSED

Files verified present:
`src/driver/rate_limit.rs`, `tests/driver_rate_limit.rs`,
`tests/fixtures/transcripts/09-rate-limit-rejected.ndjson`,
`tests/fixtures/transcripts/README.md`, `src/driver/run.rs`,
`src/driver/mod.rs`, `src/journal/mod.rs`.

Commits verified in `git log`: `b009c83`, `b362187`, `9148280`, `4c60b50`,
`8883878`.

## Notes on `actuals`

`tokens: 25000` is chars/4 over the **realized diff** — 100 162 characters across
seven files — against an estimate of 38 000. The plan's `confidence: low` was
warranted in the other direction from 20-02's: the shortfall is roughly 1.5x
rather than 6x, and its cause is that the drain-loop change turned out to be
eleven lines plus its rationale. Most of the diff is the classifier's malformed-
and out-of-range-shape coverage, which the plan's acceptance criteria enumerated
precisely, so the estimate was close on the part it could see.
