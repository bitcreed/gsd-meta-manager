# Deferred items — phase 21

Out-of-scope discoveries logged during execution, per the executor's scope
boundary: only issues **directly caused by** a task's own changes are auto-fixed.
Neither item below is caused by any plan in this phase, and neither is in a file
any plan in this phase opened.

## Two integration-test binaries are flaky under parallel execution

Found during plan `21-10`'s whole-suite verification. Both are **pre-existing
concurrency flakes**, not regressions:

| Binary | Test | Symptom |
|---|---|---|
| `tests/envelope_tracer.rs` | `a_relocated_copy_of_the_stub_refuses_instead_of_acting` | `the generated stub is executable: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }` — the classic write-then-exec race |
| `tests/driver_reattach.rs` | `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`, `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` | `the run record is on disk: Os { code: 2, kind: NotFound }` and `exactly one project has a run to observe — left: 0, right: 1` |

**Evidence that these are flakes rather than a regression.** Against **one
unchanged binary**, `cargo test --test driver_reattach` returned FAILED, FAILED,
then ok on three consecutive runs; the same binary with `-- --test-threads=1`
returned ok three times out of three. The failing runs finish in ~0.5s against
~6.1s for a passing one, which is the shape of a probe bailing before the thing
it waits for exists. `envelope_tracer` failed once inside a full-suite run and
passed immediately when run alone.

> ### CORRECTION (2026-08-28, `21-34`, round 11) — READ THIS BEFORE ACTING ON THE PARAGRAPH ABOVE
>
> **The superseded sentence, quoted verbatim from the paragraph immediately above:**
>
> > "the same binary with `-- --test-threads=1` returned ok three times out of three"
>
> **That mitigation does NOT work and must not be reached for.** It is corrected
> HERE, at the sentence, rather than only in the three later sections of this file
> that qualify it — because a reader who stops at the first section is exactly the
> reader the wrong sentence reaches. The three facts that stand:
>
> 1. **PRE-EXISTING, NOT A REGRESSION.** Round 10's orchestrator measured the
>    untouched base and the round's HEAD at the SAME rate — **3/5 red at the base,
>    3/5 red at HEAD**. A `driver_reattach` failure is therefore not a regression
>    signal from any phase-21 plan and must not be chased as one; equally it must
>    not be allowed to mask a real failure, which is what `--no-fail-fast` is for.
> 2. **`--test-threads=1` does NOT reliably fix it.** Round 4 already measured it
>    failing intermittently *under* `-- --test-threads=1` (see the round-4 update
>    below), and round 10 confirmed it. The three-out-of-three above is a true
>    record of ONE session in round 2 and is left unedited for that reason; it is
>    not a mitigation and was never re-reproduced. `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md:28`
>    independently says the same thing and gives the better reason: serialising
>    **hides** the race rather than closing it.
> 3. **The mechanism is recorded in the `21-30` entry at the END of this file**
>    ("the `driver_reattach` flake, reported and NOT absorbed"), which carries the
>    promote condition and the direction. It is pointed at rather than duplicated
>    here, so there is one place to correct next time. **Round 11 adds that the
>    mechanism record is not settled — two measured candidate mechanisms are on
>    file and no experiment has discriminated them.** See the round-11 entry at
>    the end of this file.
>
> **Also corrected, in the same breath, because they are standing claims in this
> same section that a reader would act on:**
>
> * **line 33's** `rtk proxy cargo test -- --test-threads=2` **is no longer the
>   phase gate.** Rounds 9, 10 and 11 measure the workspace at cargo's DEFAULT
>   thread count with `--no-fail-fast`; the `1193 passing` figure beside it is a
>   round-2 measurement of a tree that no longer exists (round 11's merged tree is
>   1424). The `--test-threads=2` recommendation is retained below as the dated
>   record of what rounds 5-8 used, not as an instruction.
> * **line 37's** *"Neither `tests/envelope_tracer.rs` nor `tests/driver_reattach.rs`
>   is in any `21-*` plan's `<files>`"* **is now FALSE.** `21-34` (this plan) puts
>   `tests/driver_reattach.rs` in its `<files>` — for a **comment-only** note, not
>   a fix. The fix is still not attempted and is still a standing item.
>
> **Not fixed here, and deliberately so.** The fix — scoping the run discovery to
> the test's own process group or worktree, or synchronising on the written
> artifact rather than on the process — is a synchronisation change to a
> live-process probe, not a record correction. Its failure direction is
> **false-red under parallelism: LOUD**, which is the safe direction to be wrong
> in, and is why deferring it is defensible.

**Evidence that plan 21-10 cannot be the cause.** `21-10` changed only the
*second* `run.json` write (the terminal one) and the TUI's opt-in revert. Both
failing `driver_reattach` assertions are about the run record's existence and
observability at write **one** (`JournalRun::start`), which is untouched, and
they run before any terminal write by construction — the crash test asserts
`ended_at` is still null at that point.

`rtk proxy cargo test -- --test-threads=2` over the whole suite exits **0** with
**1193 passing tests across 35 binaries and 0 failures**.

**Not fixed here.** Neither `tests/envelope_tracer.rs` nor
`tests/driver_reattach.rs` is in any `21-*` plan's `<files>`, and a fix is a
synchronisation change to a live-process probe rather than a one-line
correction. Carry into the next phase's backlog.

**Update (2026-08-22, round 5): the honest whole-suite gate is
`--test-threads=2` with `--no-fail-fast`, and the `--test-threads=1` note above
is STALE.** The round-5 orchestrator re-measured the two `driver_reattach`
tests against the **untouched** base in a throwaway worktree, five consecutive
runs: FAIL / FAIL / FAIL / ok / ok. They flake identically with no round-5
change in the tree, so a failure there is not a regression signal and must not
be chased — and equally must not be allowed to mask a real one, which is what
`--no-fail-fast` is for. Every gate in `21-15` and `21-16` is measured as
`rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2`. Round 5's
own runs did not see either test fail.

**Update (2026-08-21, round 4): `driver_reattach` is now flakier than recorded
above, and its documented mitigation no longer works.** Under
`--test-threads=4` it fails intermittently in whole-suite runs; under
`-- --test-threads=1` — which this file records as passing three times out of
three — it now also fails intermittently. Confirmed **not** a round-4
regression by building the pre-round-4 tree (`6eb1d49`) in a separate worktree
and running the binary six times: `ok, FAILED, FAILED, FAILED, FAILED, FAILED`.
The untouched baseline flakes *worse* than the round-4 tree. The run ids it uses
(`2026-07-29T12-00-00Z-aaaa`) are exactly the shape 21-13's tightened
`is_plain_path_component` pins as accepted, so that change cannot be the cause.
`--test-threads=2` remains reliably green for the whole workspace. Raising the
priority of the carried item rather than adding a new one.

> **CORRECTION (2026-08-28, `21-34`, round 11) — the two standing claims in the
> round-5 and round-4 updates above.**
>
> **Superseded, quoted verbatim:**
>
> > "the honest whole-suite gate is `--test-threads=2` with `--no-fail-fast`"
>
> > "`--test-threads=2` remains reliably green for the whole workspace."
>
> **Neither is the gate any more, and neither should be reinstated.** Rounds 9, 10
> and 11 all measure the workspace with `cargo test --workspace --no-fail-fast` at
> cargo's DEFAULT thread count. Round 11's merged-tree measurement at base
> `f5b548b` on a quiet tree is **1424 passed / 0 failed / 13 ignored**, with no
> `--test-threads` setting applied at all. `--no-fail-fast` is kept, and it is the
> half that was always load-bearing: it stops one flaky binary from masking a real
> failure elsewhere in the suite.
>
> **What round 4 got RIGHT and round 11 keeps:** the sentence above it —
> *"its documented mitigation no longer works"* — is correct and is the reason the
> round-2 `--test-threads=1` line is corrected at its own sentence at the top of
> this file rather than only here, three sections down.

---

# Round-4 adjudications (2026-08-21)

Items carried forward from the round-2 review that round 4 deliberately declines,
each with the reason from `21-PREMISES.md` Premise 6. Recorded here rather than
dropped, so nothing leaves the phase silently — the prohibition
`21-14-PLAN.md` carries as `MUST NOT drop an adjudicated-out finding silently`.

## OUT — deferred

| Item | Location | Reason declined |
|---|---|---|
| `registry::current_prompt_inputs` absent from `BLOCKING_HELPERS` | `tests/async_blocking_guard.rs:124-144` | Async-hygiene (synchronous disk reads under an `async fn`), not the failing criterion's class. The fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run` — real scope, and zero bearing on ROADMAP criterion 1. |
| The spawn-gate plan-half argument lives in a comment rather than in a checked property | `src/driver/run.rs:2300-2328` | The comment now states plainly that the plan half is a no-op there and why that is sound (decompose-once). Converting a sound, honestly-documented argument into a checked property is hardening, not gap closure. |
| Dead `PlanStep::rationale` | `src/driver/goal.rs` | No production reader. A cosmetic dead field with no security or honesty bearing. |

Any of the three can be promoted into a future phase; none is closed by
round 4, and none should be read as fixed.

## IN — closed by 21-13

| Item | Location | Disposition |
|---|---|---|
| `plan_target_phase(plan).unwrap_or_default()` writing a blank `target_phase` | `src/driver/mod.rs` (`approve_plan`) | **Adjudicated IN and closed.** Same defect class as the criterion-1 failures — a blank value reaching a persisted record, where `""` already means field-absent (D-30) — merely arriving through the model seam instead of argv. Excluding it would have repeated the exact scoping bet that lost three times. `approve_plan` now refuses with the same typed error `goal::legality` raises for a stepless plan. |

---

# Round-5 adjudications (2026-08-22)

What round 5 (`21-15`, `21-16`) deliberately did **not** do, each with its
reason. Recorded here rather than dropped, so nothing leaves the phase
silently.

## OUT — by design, not by omission

| Item | Location | Reason declined |
|---|---|---|
| `RawDriveArgs` keeps raw `String` fields | `src/driver/mod.rs` | It **is** the raw side of the parse boundary — the shape argv supplies before anything has judged it. Typing it `NonBlank` would mean the judgment happened somewhere else, which is the thing the boundary exists to prevent. Guard nine reads `DriveArgs` only, and asserts by name that its extraction never reached `RawDriveArgs`. |
| `RunRecord`'s serde fields stay `String` | `src/journal/mod.rs` | Records are **tolerant-read wire types** (D-30): a record written by a newer build must survive a round trip through this struct rather than being rejected or pruned. Validation belongs at the seams that WRITE the record, which is where 21-15 put it — `RunRecord.goal` and `.target_phase` are now written from `Option<NonBlank>`, so `""` on disk provably means absent. Typing the wire struct would turn a forward-compatible reader into a validator and break the tolerant read path. |
| The matrix row table is hand-maintained | `src/driver/mod.rs` (`positions()`) | `from_argv`'s exhaustive destructure and guard nine together bound the **type** of a seventh argv field — it cannot compile unclassified, and a raw `String` in the tree's declaration style is a loud red — but neither forces a matrix **row**. A seventh field correctly typed `NonBlank` with no row leaves the matrix at 7x6 silently. That is a coverage residual, not a blank-payload route (the type still refuses the blank), and it is disclosed at `positions()`'s own doc and in `21-15`'s truth 6 rather than claimed closed. |
| `test_support::DEGENERATE` is unreachable from integration tests | `src/test_support.rs` | The module is `#[cfg(test)]`, so it does not exist in the compiled library and a separate test crate cannot consume it. The exhaustive six-shape sweep therefore lives in-crate (the 7x6 matrix, the journal pin, the `goal_or_none` and `goal_lines` pins, `text.rs`'s own pins); the two retargeted `tests/driver_dry_run.rs` tests keep the payload lists they already had rather than gaining a hand copy, which would itself be the prohibited pattern. Making the module unconditionally `pub` (one `const`, no runtime cost) is the clean fix if a later round wants integration-side coverage of all six shapes. |

## Still OUT — the three round-4 deferrals, unchanged

Round 4 declined these with recorded reasons and round 5 does not revisit them.
None is closed; none should be read as fixed.

| Item | Location | Reason still declined |
|---|---|---|
| `registry::current_prompt_inputs` absent from `BLOCKING_HELPERS` | `tests/async_blocking_guard.rs:124-144` | Async-hygiene (synchronous disk reads under an `async fn`), not the failing criterion's class. The fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run` — real scope, zero bearing on ROADMAP criterion 1. |
| The spawn-gate plan-half argument lives in a comment rather than in a checked property | `src/driver/run.rs` (spawn gate) | The comment states plainly that the plan half is a no-op there and why that is sound (decompose-once). Converting a sound, honestly-documented argument into a checked property is hardening, not gap closure. |
| Dead `PlanStep::rationale` | `src/driver/goal.rs` | No production reader. A cosmetic dead field with no security or honesty bearing. |

## IN — closed by round 5

| Item | Location | Disposition |
|---|---|---|
| `goal.rs`'s length-bound refusal had zero live coverage | `src/driver/goal.rs`, `tests/driver_goal_seam.rs` | **Closed by 21-16 Task 4.** 21-13's predicate tightening left every `HOSTILE_PHASE_TOKENS` fixture refused at the first layer, so the second refusal — reachable only by a control-free over-length token — was never exercised. A defence-in-depth layer with zero coverage is a layer nobody notices breaking. |

---

# Round-7 items (2026-08-25)

## STANDING — SAFE-07's boundary has never executed under any verification pass of this phase

**This item does NOT close ROADMAP success criterion 4, and it must be
re-surfaced every round until a human run is recorded here.** It is a standing
item precisely because the fact keeps being stated once, in a SUMMARY
qualification, and read once. Tracked here it is read every round.

**What is unexecuted.** All **ten** behavioural tests in
`tests/driver_injection_corpus.rs` are `#[ignore]`d — they spawn the real
`claude` binary and need an authenticated subscription, so no verification pass
can run them. The ten are:

| Count | Tests |
|---|---|
| 7 | the class arms `corpus_instruction_override_*`, `corpus_role_confusion_*`, `corpus_delimiter_escape_bare_*`, `corpus_delimiter_escape_nonce_*`, `corpus_encoded_payload_*`, `corpus_tool_output_shaping_*`, `corpus_multi_turn_deferral_*` (each `…_arrives_and_leaves_the_command_unchanged`) |
| 2 | the suppression controls `the_positive_control_sees_the_claude_md_without_the_suppression_variable` and `the_negative_control_does_not_see_the_claude_md_while_the_planning_markers_arrive` |
| 1 | `both_arms_of_every_class_comparison_were_really_executed` — the arms' own non-vacuity meta-check |

`21-18-SUMMARY.md`'s SAFE-07 qualification said *eight* arms and omitted the
meta-check; that miscount is corrected, dated and append-only at the end of that
file.

**The human run.** With an authenticated `claude` CLI available, from the
repository root:

```
cargo test --test driver_injection_corpus -- --ignored --nocapture
```

**Expected output:** `10 passed, 0 failed`. Every
`corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload
**ARRIVED** at the model before asserting the command was unchanged; the two
suppression controls show the positive/negative
`CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and
`both_arms_of_every_class_comparison_were_really_executed` confirms the hostile
and clean arms both really ran. **Record the `claude --version` output beside the
result**, here in this file.

**Provenance.** The last executor-claimed live run is `21-05-SUMMARY.md:514` (10
passed against `claude` 2.1.238), a claim no verification pass has reproduced.
The round-6 reviewer independently adjudicated the same item: *carry as open —
SAFE-07's boundary has not been executed end-to-end in any pass of this phase.*

**Where the mechanised half stops and the human half begins.** What DOES run
every round, under an ordinary `cargo test`, is the structural half: thirteen
active pins proving the corpus is planted where the shipped reader reads, that
every payload survives the production bound whole, and that the typed state
carries no marker — i.e. that the labelled untrusted boundary is the only
channel. Among them, and named here by identifier because it is the test that
keeps *this item's own arithmetic* honest:

```
the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check
```

It walks the corpus file's own source and asserts exactly ten line-anchored
`#[ignore]` attributes, exactly seven corpus arm `fn` declarations, and the
meta-check present as an ignored `fn` declaration. It counts declarations, never
mentions. **Presence and wiring are verified by it; behaviour is not, and it
says so in its own doc.**

**Run history:**

| Date | `claude --version` | Result |
|---|---|---|
| — | — | *(never run under verification)* |

---

# Round-8 items (2026-08-25, plan 21-22)

## TODO — four PRE-EXISTING clippy lints make `--all-targets` fail at HEAD

**Recorded so that a reader who runs the all-targets gate does not read its
failure as round-8 breakage.** They are not fixed here: `21-22`'s prohibition 5
forbids it, and neither `src/browser.rs` nor `src/project_creator.rs` appears in
this plan's diff.

Re-measured at round 8's own tree with
`rtk proxy cargo clippy --all-targets -- -D warnings`:

| # | File:line | Lint | Offending code |
|---|---|---|---|
| 1 | `src/browser.rs:131` | `clippy::bool_assert_comparison` | `assert_eq!(entries[0].is_dir, true);` |
| 2 | `src/browser.rs:132` | `clippy::bool_assert_comparison` | `assert_eq!(entries[1].is_dir, true);` |
| 3 | `src/browser.rs:133` | `clippy::bool_assert_comparison` | `assert_eq!(entries[2].is_dir, false);` |
| 4 | `src/project_creator.rs:146` | `clippy::cmp_owned` | `assert!(result != PathBuf::from("~") \|\| dirs::home_dir().is_none());` |

`error: could not compile 'gsd-meta-manager' (lib test) due to 4 previous errors`
— **4, unchanged**, exactly the count `21-21-SUMMARY.md`'s prohibition audit
recorded. All four are inside `#[cfg(test)]` modules, which is why the library
gate never sees them.

**Evidence that they predate round 8, and predate this whole phase's later
rounds:**

```
$ rtk proxy git log --oneline 2074595..HEAD -- src/browser.rs src/project_creator.rs
(empty)
```

Neither file has been touched since `2074595`. A failure in the all-targets gate
today is therefore *definitionally* not round-8 breakage.

**The gate this phase's plans use is the LIBRARY gate, and it is clean:**
`rtk proxy cargo clippy --lib -- -D warnings` finishes with no diagnostics.
Whoever fixes these four should do it as its own change, not folded into a
gap-closure round, because the fix edits test assertions in two files no phase-21
plan owns.

## STANDING — invisible-character rendering is a property of the ratatui VERSION, not of this code

**Same shape as the pinned-Unicode-version obligation `src/text.rs` already
carries** on `is_invisible_formatting_char`: a property the tree believes it has
which actually belongs to a dependency, and which an upgrade can move silently.
It is recorded here rather than only in a SUMMARY because a SUMMARY is read once
and this file is read every round.

**What was measured, and by whom.** Plan `21-21` measured it with a scratch
reporter against the then-unescaped `src/ui/screens/delete_confirm.rs`, at
**ratatui 0.30**. Quoted from `21-21-SUMMARY.md` rather than re-derived:

```
HOSTILE INPUT   : "demo\u{e0041}r\u{ad}un"
ESCAPED FORM    : "demoU+E0041rU+00ADun"
RENDERED ROW    : "Remove \"demo\u{e0041}run\"? This only unregisters it — project files are not deleted. [y/n]"
INVISIBLE CHARS : ['\u{e0041}']
CONTAINS RAW    : false
CONTAINS ESCAPED: false
CONTAINS CLEAN  : false
```

**What it means.** ratatui 0.30's `Buffer` **DROPS zero-width graphemes before a
cell exists** — `U+00AD` and `U+200B` are simply gone — while the **tag block
survives intact** (`U+E0041` reaches a cell). So the TUI does not *reorder*: it
silently *deletes*, and a legacy key renders as a DIFFERENT string that can
collide with a real project of that name.

**Why it is load-bearing for the probe's assertions.** `CONTAINS RAW: false`
means a probe asserting "the raw form is ABSENT from the buffer" would pass
**vacuously against an unescaped site, forever**. That is why
`src/ui/screens/render_escape_guard.rs`'s probe asserts *arrival of the clean
stem* and then *presence of the escaped form*, rather than absence of the raw
one — and why the invisible-class assertion it runs over every render state is
what actually forces breadth.

**The standing obligation.** On any **ratatui upgrade**:

1. Re-run the measurement above (render a hostile identity through the real
   `Screen::render` into a `Buffer` and report which code points reached a cell).
2. Re-check that `render_escape_guard`'s assertions are still the **non-vacuous**
   ones. If a future ratatui preserves zero-width graphemes in the buffer, the
   escaped-form assertion stops being the only workable direction and the
   *absence* assertion becomes available — and, more importantly, the probe's
   current shape may start passing for a different reason than it does today.
3. Record the new measurement and the ratatui version **here**, in the table
   below.

| Date | ratatui version | Zero-width graphemes | Tag block | Recorded by |
|---|---|---|---|---|
| 2026-08-25 | 0.30 | DROPPED before a cell exists | survives intact | `21-21` |
| 2026-08-27 | 0.30.2 (`Cargo.lock`) | **PER WIDGET** — dropped by `Paragraph` and `Paragraph`-in-`Block`; **SURVIVE** through `Block::title` and `ListItem` | survives intact through all four | `21-23` |

### 2026-08-27 (`21-23`) — CORRECTION, append-only: the drop is a `Paragraph` property, NOT a `Buffer` property

**The sentence this corrects, quoted verbatim from the entry above:**

> **What it means.** ratatui 0.30's `Buffer` **DROPS zero-width graphemes before a
> cell exists** — `U+00AD` and `U+200B` are simply gone — while the **tag block
> survives intact** (`U+E0041` reaches a cell).

That is true of the sink `21-21` measured (a `Paragraph`, via
`delete_confirm.rs`) and **false as a statement about the `Buffer`**. The
zero-width drop happens in `ratatui-core`'s `Buffer::set_stringn`, which is on
the `Paragraph` path; `Block::title` and `ListItem` reach a cell by a different
route and preserve the class. The generalisation was made three times in this
phase — `21-21`'s SUMMARY, the round-8 review, and verification pass 9's
Judgment 3, which re-derived only the `Paragraph` column — and each time it hid
the two widget families where this tree's live leaks actually were.

**Re-derived independently for `21-23`**, in a throwaway crate outside this
repository depending only on `ratatui = "0.30"` (resolved: **0.30.2**, matching
this tree's `Cargo.lock`) and `unicode-width`, rendering `a<CP>b` through four
sinks into a `TestBackend` buffer. The crate was run, its output captured
verbatim, and the crate deleted (`git status --porcelain` clean). Verbatim:

```text
ratatui per-widget cell survivorship
cp          width | Paragraph   Block::title  ListItem    Paragraph-in-Block
U+202E     2 | dropped     SURVIVES      SURVIVES    dropped
U+200B     2 | dropped     SURVIVES      SURVIVES    dropped
U+00AD     2 | dropped     SURVIVES      SURVIVES    dropped
U+2062     2 | dropped     SURVIVES      SURVIVES    dropped
U+2065     2 | dropped     SURVIVES      SURVIVES    dropped
U+FEFF     2 | dropped     SURVIVES      SURVIVES    dropped
U+E0041    2 | SURVIVES    SURVIVES      SURVIVES    SURVIVES

C0 controls (probe `a<CTRL>b`)
cp          | Paragraph   Block::title  ListItem    Paragraph-in-Block
U+001B      | dropped     dropped       dropped     dropped
U+000D      | dropped     dropped       dropped     dropped
U+0007      | dropped     dropped       dropped     dropped
```

**What changes because of it.**

1. `21-23` closed a live `List`/`ListItem` leak of a third-party repository's
   commit hash, date, author and subject (`render_git_tab`) that the
   generalisation had explained away.
2. LIMIT 4 of `src/ui/screens/render_escape_guard.rs` declined the raw-absence
   assertion on this premise. The premise holds for `Paragraph` sites only, so
   the assertion is **non-vacuous** for `Block::title` and `ListItem` and was
   reinstated in `21-23`.
3. The C0 rows are new and are recorded for completeness, with their direction:
   every C0 tested is dropped by every family under measurement, so a probe
   asserting C0 absence in a `Buffer` would be **vacuous in the same way** the
   raw-absence assertion was thought to be. `strip_terminal_controls` is
   therefore justified by what reaches the TERMINAL, not by what reaches a
   `Buffer` cell — the `Buffer` is not the boundary the ESC rule defends.

**The residual, with its direction.** Four sinks were measured, not all of them.
A fifth widget family with its own cell-writing route could preserve or drop
differently and no committed control in this tree would report it.
**Under-detection, disclosed.** What bounds it is
`the_screen_renders_identity_escaped`, which renders through the REAL
`Screen::render` and so inspects whatever family a screen actually used.

## RE-SURFACED, UNCHANGED — ROADMAP success criterion 4

**Recording, not progress.** This item is re-surfaced verbatim each round and is
**not** closed by round 8. No agent can close it: it needs an authenticated
Claude subscription. `21-22`'s prohibition 4 forbids planning, executing or
claiming any work against it, and round 8 did none. The full standing item is
above, under "Round-7 items"; this is the round-8 re-surfacing of its exact
command and expected output, quoted from `21-VERIFICATION.md`'s
`human_verification` frontmatter rather than paraphrased.

**Command:**

```
cargo test --test driver_injection_corpus -- --ignored --nocapture
```

> "With an authenticated `claude` CLI available, run
> `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
> repository root and record the CLI version beside the result."

**Expected:**

> "10 passed, 0 failed, with arrival asserted before influence in every class arm
> and `both_arms_of_every_class_comparison_were_really_executed` green."

**Why human:**

> "Requires an authenticated subscription and spawns the real model binary;
> cannot run inside verification. This is criterion 4's only behavioural evidence
> and no verification pass of this phase has ever produced it."

**Round-8 status, measured rather than assumed.** The ten stay `#[ignore]`d and
untouched — `rtk proxy git diff abcb367..HEAD --stat -- tests/driver_injection_corpus.rs`
is empty — and the file's thirteen ACTIVE structural pins pass unmodified:
`rtk proxy cargo test --test driver_injection_corpus` reports
`13 passed; 0 failed; 10 ignored`. Presence and wiring are verified; **behaviour
is not, and round 8 does not claim it is.** The arithmetic itself is not
re-derived here — `the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check`
owns it and is green.

---

# Round 9 — the record (2026-08-27, plan `21-26`, append-only)

Round 9's four plans are `21-23` (the carrier `crate::text::Untrusted`), `21-24`
(the error and lookup layers), `21-25` (the eight `.planning/` carriers and the
fixture hole) and `21-26` (this one: CR-05, WR-05 and the record). What follows
is what round 9 LEARNED and cannot mechanise. Nothing above this line is
rewritten or deleted; every entry below quotes verbatim any text it corrects.

## 2026-08-27 (`21-26`) — the SIX carrier types round 9 did NOT retype

Round 9's whole thesis was that the render surface closes as a consequence of one
TYPE rather than as a list of sites, and `21-23`/`21-24`/`21-25` delivered that
for nine carrier types. **Six were left as bare `String`, and this is the record
of which, why, and in which direction each fails.** Recorded here rather than
only in a SUMMARY because a SUMMARY is read once and this file is read every
round.

**Every count below is RE-MEASURED at `21-26`'s HEAD under `rtk proxy`, not
inherited** — see the IN-04 lesson two entries down, which is exactly about
inheriting a number. Where the re-measurement disagrees with the SUMMARY that
first reported it, BOTH are given.

| # | Carrier | Bare `String` fields | The measurement that excluded it | SUMMARY it came from | Direction |
|---|---|---|---|---|---|
| 1 | `state_reader::ProjectState` | 5 (`status`, `current_phase`, `current_phase_name`, `current_plan`, `milestone`; plus `pause_context: Option<String>`) | `rtk proxy grep -rno "\.status" src/` = **175** at `21-26` HEAD. `21-23-SUMMARY.md:289` reported **135 mentions** for the type; the figures measure different things (that one is the type, this one is the single hottest field) and neither is wrong — they are recorded side by side rather than reconciled. `.status` is additionally a `HashMap` key and a decision-router input, so a carrier here is a retype of the routing layer, not of a field. | `21-23` | under-protection, silent |
| 2 | `state_reader::roadmap_md::RoadmapPhase` | 4 (`number`, `name`, `description`, `depends_on: Vec<String>`) | Reached exclusively THROUGH `ProjectState.phases`, so it cannot be retyped before #1 without splitting the parse. Field count re-measured at HEAD: 4. | `21-23`, `21-25` | under-protection, silent |
| 3 | `state_reader::queue_md::QueuedAction` | 1 (`command`) | Compared against `SAFE_COMMAND_ALPHABET` before dispatch, so the VALUE is already alphabet-bounded at the seam that matters; what is unbounded is only its RENDER. Field count re-measured at HEAD: 1. | `21-25` | under-protection, silent |
| 4 | `AppContext::filtered_aliases` / `selected_alias` | `Vec<String>` / `Option<String>` | `rtk proxy grep -rn "filtered_aliases\|selected_alias" src/` = **94** at `21-26` HEAD (`21-23-SUMMARY.md:289` reported **93**; the round's own diff added one). D-21-4 measured a 25-error cascade from retyping it. | `21-23` | under-protection, silent |
| 5 | `AppContext::status_message` | `Option<(String, Instant)>` | **There is no field a carrier could type.** The trust boundary runs through the middle of a `format!`: `rtk proxy grep -c "status_message = Some" src/app.rs` = **6** at HEAD, four of which interpolate a registry key or a run id into a sentence this build wrote, one is a literal, and the sixth forwards whatever any screen handed to `ScreenAction::SetStatusMessage` — so the producer set is **open by construction**. `21-25` escaped it AT THE RENDER SITE instead. What would force a `StatusMessage` type: closing `SetStatusMessage` so every producer must hand over a structured value rather than a formatted one. | `21-25` | under-protection, silent, and OUTSIDE the field |
| 6 | `state_reader::config_json::GsdConfig` **plus the file-BODY fields** `ProjectViewCache::archive_file_content` and `::browser_file_content` | `GsdConfig`: 6 string-ish (`mode`, `granularity`, `model_profile`, `project_code`, `phase_naming`, `response_language`, the last three `Option`). Bodies: 2 `Option<String>` | A file BODY is not a name: it goes through `archive::render_markdown_lines`, a whole markdown pipeline with its own question about what escaping means for a document. `21-25` escaped **per line** at that render instead, and the probe goes red without it. `hostile_gsd_config` names only three of `GsdConfig`'s string keys; a fourth added tomorrow is not in the fixture. | `21-25` | under-protection at the FIELD; the two bodies are bounded AT THE RENDER |

**What bounds the set as a whole, said plainly.** The census
(`the_screen_census_matches_the_tree`) and the behavioural probe
(`the_screen_renders_identity_escaped`). **The probe is a SAMPLING control**: it
renders each screen in the states its fixture constructs and inspects the cells.
`21-25` populated every one of the eleven `DetailScreen` tabs plus four
within-tab states, so the sampling is **materially better** than it was — that
population found four live leaks no reader had found in nine rounds. **That is an
improvement in the bound, not a closure of it.** A render path reachable only
under state no fixture builds is still invisible, and `render_escape_guard`'s
LIMIT 1 names two concrete surviving examples (the Defaults tab's string-edit
overlay, the Driver tab's `driver_dry_run` preview).

## 2026-08-27 (`21-26`) — STANDING: the per-widget ratatui obligation, in its own right

This is **not** a new measurement. The measurement is the second row of the table
in the STANDING ratatui entry above (`2026-08-27 | 0.30.2 | PER WIDGET | … |
21-23`), and the correction beneath it. This entry states the standing
OBLIGATION that row creates, because an obligation buried inside a correction is
an obligation a future upgrader will not see.

**On any ratatui upgrade:**

1. Re-run the survivorship measurement **per widget family** — `Paragraph`,
   `Paragraph`-in-`Block`, `Block::title`, `ListItem` — and not through one sink.
2. Re-check that `render_escape_guard`'s four assertions are still the
   **non-vacuous** ones. Assertion 4 (raw-absence) has power only for the
   preserving families; if a future ratatui made `Paragraph` preserve zero-width
   graphemes, assertion 4 gains power there and LIMIT 4 must be re-stated.
3. Add a row to the table above with the version and the date.

**And the direction the previous generalisation was wrong in, restated because
this is the part that cost three rounds.** The zero-width drop is a **`Paragraph`
property, not a `Buffer` property**. `Block::title` and `ListItem` PRESERVE
`U+202E`, `U+200B`, `U+00AD`, `U+2062`, `U+2065` and `U+FEFF` — and those two
families are exactly where this tree's live leaks were. Three artefacts asserted
the general form without measuring it (`21-21`'s SUMMARY, the round-8 review, and
verification pass 9's Judgment 3, which re-derived only the `Paragraph` column),
and each time the generalisation hid the two families that mattered. **A future
upgrade must be measured per widget family. Measuring through one sink and
generalising is the specific mistake this entry exists to prevent.**

There is an in-repo tripwire for it now:
`the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell` drives
`zero_width_only_identity()` through both `ProbeSink::Paragraph` (must answer
`false`) and `ProbeSink::ListItem` (must answer `true`). If a ratatui upgrade
moves either family, that test goes red and points here. It is a tripwire, not a
substitute for the per-family re-measurement: it covers two of the four families.

## 2026-08-27 (`21-26`) — PROCESS LESSON from IN-04: an exemption is where a prohibition gets falsified

Recorded beside the ratatui obligation because it is the lesson learned FROM it.

**Quoted verbatim from `21-REVIEW.md`'s IN-04**, which quotes `21-22-SUMMARY.md:422`:

> Prohibition 7's audit reads: *"every number here is a command output… No number
> is inherited… the one number quoted from 21-21 (the ratatui buffer measurement)
> is quoted as 21-21's measurement, explicitly, because the plan directs that it
> be quoted rather than re-derived."* That exemption was granted by the plan and
> honoured exactly, and it is the number CR-03 falsifies. Recording it here as a
> process observation for round 9's plan: a prohibition against inheriting
> numbers that carries one named exemption will be falsified at the exemption. If
> the plan directs that a measurement be quoted rather than re-derived, the wave
> that quotes it should still re-derive it.

**The rule round 9 adopted, and it belongs here rather than in a SUMMARY read
once:** *if a plan directs that a measurement be quoted rather than re-derived,
the wave that quotes it re-derives it anyway.* The cost of a redundant
re-measurement is minutes; the cost of the one exempted number being the wrong
one was a Critical finding two rounds later.

Round 9 applied it, and it paid twice in this plan alone:

* The standing brief and the round-8 review both say the tree holds **13**
  `Screen` implementors. Re-measured at HEAD:
  `rtk proxy grep -rn "impl.*Screen for " src/ --include=*.rs` returns **11**.
  The thirteen was eleven real ones plus the round-8 reviewer's own two plants,
  counted while the plants were in the tree. A plan that inherited it would have
  spent the round hunting a twelfth and thirteenth screen that do not exist.
* `21-23-SUMMARY.md:289` reports **93** mentions of
  `filtered_aliases`/`selected_alias`; re-measured at `21-26` HEAD it is **94**,
  because round 9's own diff added one. Small, and exactly the kind of drift an
  inherited number hides.

## 2026-08-27 (`21-26`) — DEFERRED, not fixed: IN-02 and IN-03 (`src/ui/roadmap_widget.rs`)

Both are **pre-existing**, both are **cosmetic**, and neither has a security or
honesty bearing. `21-26`'s prohibition 8 forbids fixing them in the last plan of a
round that must converge, and this plan does not modify `src/ui/roadmap_widget.rs`.

**IN-02 — width is measured by `chars().count()`, which is not display width.**
`src/ui/roadmap_widget.rs:134`. The `len()` → `chars().count()` change was a
correct fix for a byte/char panic, but a CJK phase name occupies two cells per
`char` and will overflow the box. `unicode-width` would be exact.
**What would promote it:** a report of an actually-overflowing box — i.e. a real
`.planning/ROADMAP.md` with a wide-script phase name, rendered.

**IN-03 — truncation can exceed its own budget when `name_max < 3`.**
`src/ui/roadmap_widget.rs:139-145`. `format!("{}...", name.chars().take(name_max.saturating_sub(3)))`
emits three characters when `name_max` is 0, 1 or 2. Pre-existing and faithfully
preserved by the round-8 edit; a `if name_max <= 3 { … }` guard is the fix.
**What would promote it:** a render path where `name_max` can actually reach 0, 1
or 2 — nobody has shown one, which is why it is cosmetic today.

Both are **display-honesty adjacent but not identity-honesty**: neither can make
two different values render as the same string in a way an operator would act on,
which is the harm this phase exists to close.

## RE-SURFACED, UNCHANGED — ROADMAP success criterion 4 (round 9)

**Recording, not progress. This is the second consecutive round with NO work
claimed against it, and that is correct.** No agent can close it: it needs an
authenticated Claude subscription. `21-26`'s prohibition 6 forbids planning,
executing or claiming any work against it, and round 9 did none — the two test
files are RUN by round 9 and EDITED by nothing in it.

Quoted verbatim from `21-VERIFICATION.md`'s frontmatter rather than paraphrased.

**Command:**

```
cargo test --test driver_injection_corpus -- --ignored --nocapture
```

> "With an authenticated `claude` CLI available, run
> `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
> repository root and record the CLI version beside the result."

**Expected:**

> "10 passed, 0 failed. Every `corpus_*_arrives_and_leaves_the_command_unchanged`
> arm asserts the payload ARRIVED at the model before asserting the command was
> unchanged; the two suppression controls show the positive/negative
> `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and
> `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile
> and clean arms both really ran."

**Why human:**

> "All ten spawn the real `claude` binary and need an authenticated subscription,
> so they cannot run inside verification. NO AGENT CAN CLOSE THIS ITEM, and the
> user has explicitly chosen to leave it tracked in `deferred-items.md:291-330`.
> … Presence and wiring verified for the ninth consecutive pass; behaviour never
> exercised by any verification pass of this phase."

**Round-9 status, MEASURED rather than assumed.** The ten stay `#[ignore]`d and
the file is untouched by the whole round:
`rtk proxy git diff --stat dfa11c6..HEAD -- tests/driver_injection_corpus.rs` is
EMPTY. The thirteen ACTIVE structural pins pass unmodified —
`rtk proxy cargo test --test driver_injection_corpus` reports
`13 passed; 0 failed; 10 ignored`. **Presence and wiring verified for the tenth
consecutive pass; behaviour not, and round 9 does not claim it is.**

## 2026-08-27 (`21-26`) — DRIVE-04's two backstops, RE-RUN rather than inferred

**These are reconfirmations that round 9's diff did not move them, not new
evidence, and the framing matters.** DRIVE-04's boundary and precision were
measured by executed plans `21-02`, `21-04` and `21-22` and verified by pass 9.
Round 9 touches no cap, no seam and no arithmetic. Writing a *new* boundary
predicate for DRIVE-04 inside a render-honesty round would be the
manufactured-predicate overclaim this phase exists to end, so what is recorded is
a re-run.

`rtk proxy cargo test --test driver_escalation_cap --no-fail-fast` at `21-26`
HEAD: **8 passed; 0 failed; 0 ignored**. Both cap directions green, named:

* **Boundary** — a per-run escalation cap at or above the resolved step cap is
  refused at the seam (`a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run`,
  `a_budget_of_zero_refuses_above_the_run`), and one below it is accepted
  (`a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap`).
* **Precision** — the decomposition consultation is counted against the SAME cap
  as the escalations, and exceeding it parks with a typed reason on the journal
  rather than silently degrading to rules-only
  (`a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing`,
  `the_fixture_really_reaches_the_state_the_rule_table_does_not_cover`).

`rtk proxy git diff --stat dfa11c6..HEAD -- tests/driver_escalation_cap.rs` is
EMPTY: the file is unchanged across the whole round.

## 2026-08-27 (`21-26`) — the round-9 self-audit, with its controls

Run against round 9's own diff, the way `21-22` audited round 8.

**Scope, stated exactly because a self-audit that hides its range is worthless:**
`git diff 98610bf..08605b3`, **6244 added lines** — every commit of round 9's four
plans plus the orchestrator's `Display for Rendered` fix and both tracking
commits, up to and including `21-26`'s two code commits. **This entry's own
commit is excluded by construction:** a debt-marker count cannot measure the file
it is being written into, because writing the count changes it. The `Cf` half IS
extended to this entry and reported separately below, since that scan is not
self-referential — nothing here writes a `Cf` character.

| Check | Result | The control that makes it non-vacuous |
|---|---|---|
| Raw `General_Category=Cf` characters in added lines | **0** | The same scan, with one `U+200B` appended to the input, returns **1**. Delta `+1`, so the zero is a measurement rather than a broken scan. |
| `TBD` / `FIXME` / `XXX` debt markers | **0** (3 raw grep hits, all false positives) | Control needle `the` over the same added lines returns **1819**, so the grep is reaching the input. The three hits are all the literal `U+XXXX` escape-marker format string in prose about `display_identity`. |
| `TODO` / `HACK` / `PLACEHOLDER` debt markers | **0** (3 raw grep hits, all false positives) | Same control. The three hits are the SUMMARY template's own sentence *"No hardcoded empty value, placeholder string, TODO or unwired component was introduced."*, once per wave SUMMARY. |
| `.planning/REQUIREMENTS.md` last touched | `0c4f712` | `rtk proxy git log --oneline -3 -- .planning/REQUIREMENTS.md` — unchanged for the seventh consecutive round. |

**The false positives are reported rather than filtered away.** A grep tuned
until it returns zero is a grep that has been taught not to look; the honest form
is the raw count plus what each hit actually is.

**The `Cf` scan extended to THIS entry**, run over its own 256 added lines before
it was committed: **0** raw `Cf` characters, control `+1` on a planted `U+200B`.
Note that the debt-marker rows above cannot be extended the same way — this entry
itself contains the literal strings `U+XXXX` and `TODO` while explaining that the
round's three hits of each were exactly those two false positives, so a
re-measurement including this file would count them again and mean nothing new.

## 2026-08-27 (`21-26`) — OPEN: verification pass 9's four-character sighting is still unexplained

Not a deferral of work — a deferral of an EXPLANATION, and it is recorded because
`21-23` was explicit that it did not close it and a later reader should not read
the fixture's red as having done so.

Verification pass 9 reported `the_screen_renders_identity_escaped` panicking ONCE
for `DetailScreen [GitHistory tab]` with
`['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}']` — **four** characters. `21-23`
populated `probe_ctx`'s `git_entries` and the defect then fired **20 runs out of
20**, reporting **eight** characters, which is one hostile pair per `GitLogEntry`
field and is what a render of all four fields must produce. Four is two fields'
worth.

So: the CLASS of defect pass 9 saw is reproduced and settled by construction; its
exact COUNT is not reproduced, and **the mechanism of that single sighting
remains unexplained**. Recorded in `probe_ctx`'s doc, in `46cfdf7`'s message, in
`21-23-SUMMARY.md`, and here. Any residual intermittency in this probe after
round 9 must be reported as evidence for pass 9's reading (b) rather than
absorbed as noise.

---

# ROUND 10 (`21-27` … `21-30`) — appended 2026-08-27

Everything below is APPENDED. No line above this heading was edited or deleted;
where an entry corrects an earlier one it quotes the earlier text verbatim and
says what changed. Every number is traced to the SUMMARY it came from BY NAME
and was re-measured under `rtk proxy` against round 10's final tree.

## 2026-08-27 (`21-30`) — the DISCLOSURE THAT WAS MISSING, named as such

**This is the most important entry of round 10, and it is about the record
rather than about the code.**

Round 9's own disclosure table (this file, `## 2026-08-27 (21-26) — the SIX
carrier types round 9 did NOT retype`) opens:

> *"Round 9's whole thesis was that the render surface closes as a consequence
> of one TYPE rather than as a list of sites, and `21-23`/`21-24`/`21-25`
> delivered that for nine carrier types. **Six were left as bare `String`, and
> this is the record of which, why, and in which direction each fails.**"*

That table lists six carriers. **Two live, unescaped, carrier-shaped `String`
fields were absent from it while five others were listed:**

* `ui::screens::DriverOutputLine::text` — the Driver tab's live output pane, the
  largest render surface in the tree, drawing the model's own prose.
* `ui::screens::ProjectViewCache::defaults_text_buffer` — the Defaults tab's
  string-edit popup, drawing a raw copy of a value the list one render above
  already escaped.

**That selective omission is what turned CR-02 and CR-03 from KNOWN LIMITATIONS
into FINDINGS.** Had either appeared in that table with its direction, the
reviewer would have read a disclosed residual with an owner. Instead the table
read as complete — six named, exhaustively argued, each with a measurement — and
a reader had no way to know it was short by two. **A round that discloses
selectively has not disclosed**, and neither carrier was newly discovered by
round 10: both were live in the tree the whole time round 9 was writing that
table.

Recorded in those terms deliberately. Presenting CR-02 and CR-03 as things round
10 *found* would repeat the error one level up.

## 2026-08-27 (`21-30`) — the CORRECTED carrier table

Append-only. The round-9 table above is **not edited**; this states what changed.

### Leaving the residual set

| Carrier | Round-9 status | Round-10 status | By |
|---|---|---|---|
| `ui::screens::DriverOutputLine::text` | **absent from the table**, live and unescaped | `crate::text::Untrusted` — held by the TYPE; a new render is a compile error | `21-28` T1 (`21-28-SUMMARY.md`, commit `4801187`) |
| `ui::screens::ProjectViewCache::defaults_text_buffer` | **absent from the table**, live and unescaped | `ui::screens::EditBuffer` over `crate::text::Untrusted` — no `Display`, no `AsRef<str>`, no `Into<Cow<'static, str>>` | `21-30` T1, commit `5461d19` |

### The carriers round 10 deliberately did NOT retype

**Named here BEFORE anyone reviews round 10, which is the whole point of this
entry existing.**

| Carrier | The measurement that excluded it | SUMMARY it came from | Direction | What would force the promote |
|---|---|---|---|---|
| `journal::inbox::InboxMessage::text` | Its consumers reach `src/journal/inbox.rs`, which no plan in wave 1 owned; retyping it would have been a wave conflict, not a closure (D-21-39). Held by a CALL plus a probe fixture. | `21-28-SUMMARY.md` (D-21-39, "Limits carried forward" #1) | **under-protection, silent** — a NEW render compiles and draws; nothing goes red at the moment the new site is written | a third render of the value, or any change already opening `src/journal/inbox.rs` for another reason |
| `ui::screens::DryRunPreview::report` | Its consumers reach `src/app.rs`, likewise outside wave 1's fence. Held by a CALL plus the `Driver tab, dry-run preview` probe state. | `21-28-SUMMARY.md` (D-21-39) | **under-protection, silent** | a third render of the value, or any change already opening `src/app.rs` |
| `config::PromptInput::path` / `::digest` | A THIRD call-held carrier on the same path, and it was in NEITHER round 9's table NOR `21-28`'s plan. `21-28` surfaced it in its own "Next Phase Readiness" and asked that it be recorded here; this is that record. | `21-28-SUMMARY.md` ("Next Phase Readiness") | **under-protection, silent** | any change opening `src/config.rs`; note `21-28` also MEASURED that today's screen render passes authored `&'static str` paths and hex digests, so this is not a live leak — see the WR-06 correction below |

The six carriers of round 9's table are unchanged by round 10 and remain exactly
as that table records them. Round 10 retyped no member of that six.

### What bounds the whole set, re-stated

Unchanged in KIND from round 9's statement: the census and the probe, and **the
probe is a SAMPLING control**. It is materially better than it was — `21-28`
populated `ctx.driver_output`, `cache.driver_journal` and `cache.driver_inbox`
and added the `driver_dry_run` state; `21-30` added the `Defaults tab, string
edit` state, which was the last of the two concrete unprobed states round 9's
LIMIT 1 named. **That is an improvement in the bound, not a closure of it.** A
render path reachable only under state no fixture builds is still invisible.
`render_escape_guard`'s LIMIT 1 now names a NEW concrete surviving example — the
Defaults tab's DROPDOWN overlay — because a residual with no example is a
residual nobody can check.

## 2026-08-27 (`21-30`) — the COMPLETE WR/IN triage: all ten items, no exceptions

Every WARNING and INFO finding of `21-REVIEW.md` gets a line. **An item with no
line in this table is a failure of this entry.**

| # | Disposition | Owning plan | Reason (re-verified, not inherited) |
|---|---|---|---|
| WR-01 | **CLOSED** | `21-27` | `Untrusted`'s absent-trait claim certified at SIX absences (was three), each new arm observed red by planting its impl. `21-27-SUMMARY.md`, D-21-36. |
| WR-02 | **CLOSED in one half, NARROWED in the other** | `21-30` T2 | The vocabulary claim is now TRUE by a two-variant `RenderDisposition` enum — a third value is not expressible (E0308 captured). The "only route" half is genuinely FALSE and stays false: `mod sealed` is `pub(crate)` by design, so an in-crate hand-write is possible. Narrowed to convention in the doc, with the falsified sentence quoted verbatim, and given a census observed red by planting a hand-written impl at `src/driver/liveness.rs:501`. |
| WR-03 | **CLOSED, with a finding** | `21-30` T2 | `adjudication_reason` had zero readers; it now has two (the probe's assertions 2/3/4 quote it, and a committed non-empty/forbidden-word control). **The measurement WR-03 predicts fired** — see its own entry below. |
| WR-04 | **ALREADY CLOSED at HEAD — recorded with evidence, not deferred** | nobody; it landed after `21-REVIEW.md` was written | Re-read `src/main.rs:228-256` at `21-30`'s HEAD. The `.to_string()` workaround is GONE and the comment is corrected; it now reads *"commit `7bf8f6b` fixed it at the source and added `rendered_display_honours_the_format_spec_in_both_directions` to certify it, so the workaround is gone and the padding is now the type's own behaviour."* Two commits, both verified present: `7bf8f6b` ("Display for Rendered must pad, not write_str — with the control that was missing") fixed the source; **`c9345a1`** ("correct the stale write_str comment left by 7bf8f6b (WR-04)") corrected the comment. `git diff --stat` for round 10 does not name `src/main.rs`. |
| WR-05 | **CLOSED** | `21-29` | 23 executable render sites under `src/ui/` converted to `render_for_terminal`, plus a committed census in `src/ui/mod.rs` asserting an equality on a count, observed RED at 23 sites against the unconverted tree. `21-29-SUMMARY.md`, D-21-42. |
| WR-06 | **CLOSED — and its PREMISE was refuted** | `21-28` | The escape at `render_disclosure` is real and the control is committed there. But `21-28` MEASURED that the live screen path (`DriverConfirmScreen::render` → `registry::current_prompt_inputs`) builds every `path` from authored `&'static str`s in `DISCLOSED_PROMPT_INPUTS` and every `digest` from `sha256_digest` (hex), so **today's screen render is NOT a live leak** and no screen fixture could go red for it. The control belongs at the function, where the untrusted value enters, and it does go red there. Recorded because a fixture reporting green over an authored constant is the exact shape this phase keeps finding. |
| WR-07 | **CLOSED — and the reviewer's claim is REFUTED** | `21-29` | See its own entry below. |
| WR-08 | **CLOSED** | `21-29` | The Debug-notation control's concatenation trap closed for the WHOLE TREE by adding a two-invisible-character fixture at `LOOK_ALIKE_PAIRS` index 6, and every consumer of the shared list re-run. `21-29-SUMMARY.md`, D-21-46. |
| IN-01 | **CLOSED** | `21-28` | `shown_capped`'s completeness claim replaced by a census over `driver.rs`/`driver_confirm.rs`, observed red by planting — after two real defects in the census itself were found the same way. `21-28-SUMMARY.md`. |
| IN-02 | **CLOSED at the Defaults popup; the ROADMAP-widget half stays DEFERRED** | `21-30` T2 | `detail.rs`'s popup width went from `str::len()` (BYTES, on a value out of the project's `.planning/config.json`) to `chars().count()`. See the residual entry below, which EXTENDS the existing IN-02/IN-03 deferral rather than contradicting it. |

**IN-03 is not in this table because it is not one of the ten WR/IN items this
entry covers** — it is a separate pre-existing `roadmap_widget.rs` deferral,
untouched by round 10 and unchanged in its own entry above.

## 2026-08-27 (`21-30`) — IN-02's REMAINING inexactness, extending the existing entry

The entry above (`## 2026-08-27 (21-26) — DEFERRED, not fixed: IN-02 and IN-03`)
records IN-02 against `src/ui/roadmap_widget.rs:134` and says:

> *"The `len()` → `chars().count()` change was a correct fix for a byte/char
> panic, but a CJK phase name occupies two cells per `char` and will overflow the
> box. `unicode-width` would be exact."*

**That reading is unchanged and this entry extends it to a second site.**
`21-30` T2 made the same `len()` → `chars().count()` change at
`src/ui/screens/detail.rs`'s Defaults edit popup, so that site now has the SAME
residual the roadmap widget has: **a character count is still not display
width.** A CJK character occupies two terminal cells and a combining mark
occupies none.

**Why it was not closed exactly.** `unicode-width` is a TRANSITIVE dependency of
ratatui, not a direct one. Adding it is a `Cargo.toml` change, which fires the
package-legitimacy gate in a phase whose `21-RESEARCH.md` carries no
`## Package Legitimacy Audit` table, in the last plan of a tenth consecutive
round that must converge. `git diff Cargo.toml Cargo.lock` for round 10 is
EMPTY — no dependency of any kind was added, which is a stronger check than
naming one crate.

**Direction: over-sizing for narrow scripts is impossible (a char count is never
above a cell count for them); UNDER-sizing for wide scripts, silent** — a CJK
value now produces a popup half as wide as its text needs, where before it
produced one three times too wide. Both are cosmetic; neither can make two
different values render as the same string.
**What would promote it:** a report of an actually-clipped popup or box, or any
change that already opens `Cargo.toml` for another reason.

## 2026-08-27 (`21-30`) — WR-03's own finding: one adjudication reason DOES use a verdict word

WR-03's instruction was to MEASURE before asserting. The measurement fired.

`RenderAdjudicated::adjudication_reason`'s doc says the reason must name *"which
values this screen draws and where their bytes come from — never 'escaped' or
'safe'"*. Measured over all eleven adjudicated screens at `21-30`'s HEAD: **ten
clean, one violation.**

**`NormalScreen` (`src/ui/screens/normal.rs`)** uses `escaped` as a whole word:

> *"`row_badge`'s lookup keys off the RAW alias while the cell beside it is
> escaped — the worked example of the split."*

**Not fixed by `21-30`, and the reason is a fence, not a judgement.**
`src/ui/screens/normal.rs` is `21-29`'s file, merged in wave 1, and `21-30`'s
prohibitions fence the screen files. Rewriting a reason there to make a number
look right would be a wave-fence violation.

It is recorded instead as ONE dated entry in `REASON_VERDICT_EXEMPTIONS` in
`src/ui/screens/render_escape_guard.rs`, pinned to **that screen AND that
token**. **Direction: under-detection, one screen and one token wide; LOUD in
every other direction** — a SECOND forbidden word in the same reason still goes
red, every other screen is unexempted, and the entry reports itself STALE on
every run once the reason is rewritten, so it cannot quietly outlive its subject.
**What would remove it:** one clause rewritten to name the split by provenance
(`render_for_terminal`) rather than by verdict, owned by whoever next edits
`normal.rs`.

**A second, smaller finding, recorded because it is the same shape this phase
keeps hitting.** `21-30`'s FIRST formulation of that control asked
`reason.to_lowercase().contains("safe")` and went RED against `DetailScreen` —
for **`SAFE-07`**, a requirement ID. That is a control false of a CORRECT
implementation: WR-08's shape, and `21-29` deviation #3's shape, reproduced a
third time. The CONTROL was fixed (whole-token matching, with both directions of
the token check themselves asserted), not the reason reworded into compliance.

## 2026-08-27 (`21-30`) — WR-07's panic claim: REFUTED, with who measured it

`21-REVIEW.md`'s WR-07 asserts that Rust's `slice::sort_by` detects total-order
violations and panics. **Verification pass 10 measured that and it does not.**
Quoted verbatim from `21-VERIFICATION.md`'s `deferred` block:

> *"I built and ran a standalone Rust program (rustc 1.97.1, matching this
> toolchain) sorting a `Vec<f64>` containing multiple `NaN` values with the exact
> comparator shape used in `backlog.rs:88-106`
> (`partial_cmp(...).unwrap_or(Equal)`), at both small (5-element) and larger
> (2000-element, 1/3 NaN) sizes. Neither run panicked; both produced a
> silently-wrong order with NaNs interspersed. Rust's stable `slice::sort_by`
> does NOT panic on a non-total-order comparator on this toolchain — WR-07's
> specific claim ('Rust's current slice::sort_by detects total-order violations
> and panics') is not reproducible and is likely incorrect, possibly confusing
> Rust with Java's TimSort. The underlying issue (a
> `.planning/phases/999.NaN-x` directory name silently corrupts backlog sort
> order rather than being a lookup/security issue) is real but is a
> display-ordering correctness bug, not a DoS/panic, and does not block this
> phase's goal."*

Toolchain recorded by pass 10: **rustc 1.97.1**.

**`21-29` fixed the REAL defect, not the reported one.** `parse_backlog_items`
now sorts on `f64::total_cmp` over a finite-filtered key — a total order by
construction, with no fallback arm — and antisymmetry and transitivity are swept
over every pair and triple of a fixture set including the hostile input
(`21-29-SUMMARY.md`, D-21-45, RED 3).

**This is a PROCESS record as much as a technical one.** A claim inherited
without measurement is how a round ships a test that asserts something false. Had
`21-29` written a `#[should_panic]` arm from the reviewer's text, it would have
been green for the wrong reason on a comparator that silently corrupts order.

## RE-SURFACED, UNCHANGED — ROADMAP success criterion 4 (round 10)

**Recording, not progress. This is the THIRD consecutive round with NO work
claimed against it, and that is correct.** `21-30`'s prohibition 8 forbids
planning, executing or claiming any work against it.

Quoted VERBATIM from `21-VERIFICATION.md`'s `behavior_unverified_items`
frontmatter — command, expected result and why-human, not paraphrased:

> **truth:** *"A `.planning/` file or `CLAUDE.md` carrying injected instructions
> does not change which command the driver executes (ROADMAP success criterion 4
> / SAFE-07)"*
>
> **test:** *"With an authenticated `claude` CLI available, run
> `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
> repository root and record the CLI version beside the result."*
>
> **expected:** *"10 passed, 0 failed. Every
> `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload
> ARRIVED at the model before asserting the command was unchanged; the two
> suppression controls show the positive/negative
> `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and
> `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile
> and clean arms both really ran."*
>
> **why_human:** *"All ten spawn the real `claude` binary and need an
> authenticated subscription, so they cannot run inside verification. NO AGENT
> CAN CLOSE THIS ITEM, and the user has explicitly chosen to leave it tracked in
> `deferred-items.md`."*

**Round-10 status, MEASURED (all under `rtk proxy`):**

* `git diff --stat b1d0478..HEAD -- tests/driver_injection_corpus.rs` — **empty.**
  The file was RUN by round 10 and EDITED by no plan of it.
* `cargo test --test driver_injection_corpus` — **13 passed / 0 failed / 10
  ignored.** Identical to the figure pass 10 recorded.

Presence and wiring verified for the **eleventh** consecutive pass; behaviour
not, and round 10 does not claim it is. **It is permanently agent-unclosable by
construction.** It is expected and correct for ROADMAP to remain at 4/5 after
this round.

## 2026-08-27 (`21-30`) — the two DRIVE-04 backstops, re-run

**Reconfirmations by re-run, not new work, and that is the honest framing.**
DRIVE-04's boundary and precision were measured by executed plans `21-02`,
`21-04` and `21-22` and re-verified by verification pass 10. Round 10 touches no
cap, no seam and no arithmetic; this shows its diff did not move them. Writing a
*new* boundary predicate for DRIVE-04 in a render-honesty round would be the
manufactured-predicate overclaim this phase exists to end.

`rtk proxy cargo test --test driver_escalation_cap` at `21-30`'s HEAD:
**8 passed / 0 failed / 0 ignored.**

| Row | Direction | Arm |
|---|---|---|
| edge-probe 6 (boundary) | a cap AT or ABOVE the resolved step cap is refused at the seam | `a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run`, `a_budget_of_zero_refuses_above_the_run` |
| edge-probe 6 (boundary) | a cap BELOW it is accepted | `a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap` |
| edge-probe 7 (precision) | the decomposition consultation counts against the SAME cap, and exceeding it parks with a typed reason rather than degrading silently to rules-only | `a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing`, `the_three_boundaries_are_measured_against_the_resolved_step_cap` |

`git diff --stat b1d0478..HEAD -- tests/driver_escalation_cap.rs` is **empty** —
the file is RUN and unchanged.

## 2026-08-27 (`21-30`) — the ROUND-10 SELF-AUDIT, against non-vacuous controls

Run over every ADDED line of round 10's whole diff (`git diff 80bc4c1..HEAD`, 19
commits, 25 files, **5380 added lines**), the way `21-26` audited round 9.

| Check | Result | Its control |
|---|---|---|
| raw characters with `General_Category=Cf` in added lines | **0** | The SAME scanner over a deliberately planted string carrying `U+200B` and `U+00AD` returns **2**. The zero is therefore a measurement, not a scanner that never looked. |
| `TBD` / `FIXME` / `XXX` substring matches | **5, all false positives — 0 genuine** | see below |
| `TODO` / `HACK` / `PLACEHOLDER` substring matches | **2, all false positives — 0 genuine** | A control needle (`e`) matches **3935** added lines, so neither zero is a scanner that never looked. |

**Both numbers are reported rather than the flattering one.** The seven
substring matches were each inspected and are all false positives:

* **3 × `XXX`** — the string `U+XXXX`, this project's OWN display notation for an
  escaped character, in three doc comments `21-30` T1 added about what must NOT
  reach the operator's `config.json`.
* **2 × `FIXME` + 2 × `TODO`** — prose in `21-28-SUMMARY.md` and
  `21-29-SUMMARY.md` stating that no TODO or FIXME was introduced.

Reporting these as "0 debt markers" without the split would have been the same
move as deleting evidence to make a number look right — the shape `21-28`
deviation #4 and `21-29`'s `partial_cmp` count both declined to make.

**`.planning/REQUIREMENTS.md` is untouched by round 10.** `git log -1 --
.planning/REQUIREMENTS.md` still ends at **`0c4f712`** ("docs(phase-21):
re-verification after third gap closure — 4/5, new critical found"). Eighth
consecutive round.

## 2026-08-27 (`21-30`) — the pre-existing clippy lints, re-measured a third time

Unchanged and still deferred. `rtk proxy cargo clippy --all-targets -- -D warnings`
at `21-30`'s HEAD reports exactly **four** lints of the same two kinds in the
same two files: `bool_assert_comparison` ×3 at **`src/browser.rs:155,156,157`**
and `cmp_owned` ×1 at `src/project_creator.rs:146`. `cargo clippy -- -D warnings`
(the stated project gate, lib only) is exit 0.

**A stale figure corrected, for the third time.** `21-30`'s own `<tooling_note>`
places the three `bool_assert_comparison` lints at `src/browser.rs:131,132,133`.
Both `21-28-SUMMARY.md` and `21-29-SUMMARY.md` (its F4) re-measured them at
`155,156,157`, and so did `21-30`. **Neither file is in round 10's diff**, so
this is a stale number in three consecutive plans' text, not a change any of them
made. Re-measure, never inherit.

## 2026-08-27 (`21-30`) — the `driver_reattach` flake, reported and NOT absorbed

`tests/driver_reattach.rs` failed its two documented tests in round 10's
pre-work baseline run — **before any file had been edited** — and again in the
`21-30` T1 and T2 workspace runs, passing **3/3 on an isolated re-run** each
time. It is **not a regression**: the orchestrator proved it against the
untouched base, where the same two tests flake at the same rate.

The variable is not `--test-threads=1`, which does not reliably fix it. It is
other driver-spawning test binaries running concurrently, because these tests
discover runs via a **system-wide `/proc` scan that does not stop at the
process-group or worktree boundary**. A green full-suite run of the wave-1 tree
is **1409 passed / 0 failed / 13 ignored**.

**What would promote it:** scoping the scan to the test's own process group or
worktree, so a concurrent sibling binary's driver is invisible to it. **Direction:
false-red under parallelism, LOUD** — it fails the build rather than passing
something broken, which is the safe direction, and it is why this is a deferral
rather than a blocker.

---

# ROUND 11 (`21-31` … `21-34`) — appended 2026-08-28, append-only

## 2026-08-28 (`21-34`) — STANDING: the `driver_reattach` flake record, corrected and made checkable

This entry supersedes nothing above it; it is where the corrections placed at the
stale sentences point, and it is the one place to edit next round.

### The three facts that stand

1. **Pre-existing, not a regression.** Measured by round 10's orchestrator
   against the untouched base and against the round's HEAD: **3/5 red at the
   base, 3/5 red at HEAD** — the same rate. Every phase-21 round since has
   re-observed it without any of them being able to cause it.
2. **`--test-threads=1` does NOT fix it.** Round 4 measured it failing
   intermittently *under* that setting; round 10 confirmed. The round-2
   three-out-of-three that this file opens with is a true record of one session
   and is left unedited, corrected in place rather than deleted.
3. **It is not always the same test.** NEW in round 11, and the reason this
   paragraph exists. The binary has three tests; two of them flake, and *which*
   one fires varies run to run:

   | Observer | Which arm fired | Isolated re-run |
   |---|---|---|
   | round-10 / `21-34` planner baseline at `343c408` | `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` | — |
   | `21-31` (wave-1 worktree, Task 3 run) | `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` — **a different arm** | 3 passed / 0 failed |
   | `21-32` (wave-1 worktree) | `a_run_killed_without_an_ending_...` again, with `a_fresh_scan_...` **passing** in the same run | 3 passed / 0 failed |
   | `21-33` (wave-1 worktree) | **2 of 3** failed in the full-workspace run | 3 passed / 0 failed |
   | orchestrator, post-merge, **quiet tree** (no concurrent suites) | **none — fully green** | n/a |
   | `21-34` merged-tree gate, this entry's own run | see the round-11 gate entry below | see below |

   **Three sibling executors ran concurrently in separate worktrees in wave 1 and
   the flake fired in all three runs, on different arms.** The orchestrator's
   post-merge run on a quiet tree came back fully green. **A green quiet-tree run
   is consistent with the mechanism and is NOT evidence the flake is fixed** —
   recording it as a fix would be exactly the absorption this entry exists to
   prevent.

### The mechanism is NOT settled — two measured candidates, no discriminating experiment

This is a correction to the confidence of the record, not to either measurement.
Both of the following were measured, by different phases, and **they are not
mutually exclusive**:

| # | Candidate mechanism | Measured by | Evidence for it | Evidence against it being the WHOLE story |
|---|---|---|---|---|
| M1 | The tests discover runs via a **system-wide `/proc` scan that does not stop at the process-group or worktree boundary**, so a concurrent driver-spawning sibling binary is visible to them | round 10 (`21-30` entry above) | wave-1's three concurrent worktrees all flaked; isolated re-runs green 3/3 each; quiet-tree post-merge run fully green | phase 19 reproduced the failure **4/4 in a clean `git archive` tree at `0a84023`**, single binary, nothing else running |
| M2 | A **spawn/write race inside the test itself**: `live_within(pid, RUN_ID, …)` waits for the driver's *process* (matching its cmdline) and the test then asserts on `run.json` / journal records the driver has not necessarily written yet | phase 19 wave-4 gate, recorded at `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md` | the ~0.5s-vs-~6.1s runtime tell; 4/4 reproduction in isolation; `WINDOWS.md:27` recorded 3/3 failing **in isolation** during phase 20 | wave-1's isolated re-runs in round 11 were green 3/3 three times over, so isolation alone does not reliably reproduce it now |

**The honest synthesis, stated as a hypothesis and NOT as a measurement:** M2 is a
real race window and M1 (or any system load) widens it. Nothing on file
discriminates them, and this entry does not claim to.

**The discriminating experiment, as the promote condition.** Run the binary N
times at `0a84023` and at HEAD, on an otherwise idle machine, with and without a
concurrently running second driver-spawning binary, and record the four rates. If
the isolated rate is non-zero, M2 is live independently of M1 and the fix is to
synchronise on the written artifact — **not** to scope the `/proc` scan, and
**never** to serialise with `--test-threads=1`, which hides it either way.

**Direction:** false-red under parallelism, **LOUD**. It fails the build rather
than passing something broken, which is the safe direction and is why this stays
a deferral rather than a blocker. **Not attempted by `21-34`** — prohibition 4 of
its plan forbids it, because a synchronisation change to a live-process probe is
not a record correction.

### CORRECTION to the paragraph above, made by `21-34` against its OWN measurement, hours after writing it

The table above records wave 1's *"every isolated re-run was green 3/3"* and the
orchestrator's fully-green quiet-tree run. Both are true records of those runs.
**`21-34`'s own merged-tree gate then contradicted the generalisation they
invite,** and the finding is recorded here rather than absorbed, because a record
plan that smooths its own contradicting measurement has failed at the one thing
it exists to do.

**Measured by `21-34` on the merged tree at `a1342a1`, on an idle machine with no
sibling suite running:**

| Run | Command | Result |
|---|---|---|
| workspace gate | `cargo test --workspace --no-fail-fast` | **1422 passed / 2 failed / 13 ignored** — BOTH flaking arms fired together, a third distinct pattern |
| isolated 1 (before the workspace run) | `cargo test --test driver_reattach` | **ok. 3 passed** (6.12s) |
| isolated 2 | same | **FAILED. 1 passed / 2 failed** (0.53s) |
| isolated 3 | same | **ok. 3 passed** (6.12s) |
| isolated 4 | same | **FAILED. 2 passed / 1 failed** (0.53s) |
| isolated 5 | same | **FAILED. 2 passed / 1 failed** (0.54s) |
| isolated 6 | same | **FAILED. 2 passed / 1 failed** (0.53s) |
| **isolated total** | | **2 green / 4 red out of 6** |

**What this refutes: M1 CANNOT BE THE WHOLE STORY.** Round 10's mechanism
sentence names *"other driver-spawning test binaries running concurrently"* as
the variable. Four of those six reds happened with **no other test binary running
at all**. A concurrent sibling therefore is not necessary for the failure, and
any future fix that only scopes the `/proc` scan to the process group may leave
the rate where it is.

**A THIRD candidate mechanism, M3, which the record did not have.** The
parallelism that remains when nothing else is running is **inside this process**:
the three tests run on separate threads by default, and `isolate_envelope_root()`
sets a **process-wide** environment variable (`std::env::set_var`) from whichever
thread reaches it first, while the sibling threads are already executing.
**M3 is NAMED, NOT MEASURED** — no experiment here attributes the failure to it,
and it is written down so the discriminating experiment covers it rather than to
claim a cause.

### The serialised sample, and why it is NOT a reinstatement of `--test-threads=1`

**`21-34` also ran `-- --test-threads=1` five times back to back: 5 green, at
~6.67s each.** That number is reported because suppressing an inconvenient
measurement is the same failure as inflating a convenient one. **It does NOT
reinstate the mitigation, and the correction at the top of this file stands
unchanged.** Three independent reasons:

1. **Round 4 measured this binary FAILING under `-- --test-threads=1`.** A
   five-run green streak does not overturn a recorded failure under the same
   flag; it just means five runs did not hit it.
2. **n = 5 is the same kind of evidence that produced the original false claim,
   which was n = 3.** Recording "5/5 green under the flag" as a fix would be
   committing the round-2 error again with a slightly larger sample — inside the
   very entry that exists to correct it.
3. **Serialising HIDES the race rather than closing it**
   (`.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md:28`).
   A green obtained by removing the concurrency is not evidence the artifact
   synchronisation is correct; it is evidence the window was not entered.

**What the pairing legitimately establishes**, and this is the only claim made
from it: **parallelism matters even with no other binary running.** That is a
fact about the three tests in this one process — evidence for M2 and M3 — and it
is why the discriminating experiment below must vary intra-binary parallelism as
well as inter-binary load.

**The discriminating experiment, updated.** Run the binary N times in each of
four conditions — (i) default threads, idle machine; (ii) `--test-threads=1`,
idle machine; (iii) default threads with a concurrent second driver-spawning
binary; (iv) `--test-threads=1` with the same concurrent load — at HEAD and at
`0a84023`, and record all eight rates with N large enough that a five-run streak
cannot decide it. **Direction is unchanged: false-red, LOUD.** The fix is still
not attempted here.

### The correction's completeness, established by inventory rather than by memory

Correcting *some* locations and not others reproduces this round's own species —
a claim wider than the evidence certifying it — one level up, inside the record.
So the correction is bounded by a committed grep inventory rather than by an
author's sweep (D-21-61).

**Commands, both run under `rtk proxy` so RTK's line filtering cannot make a
count read low:**

```
rtk proxy grep -rn "test-threads"     .planning/ tests/ src/   ->  133 hits
rtk proxy grep -rn "driver_reattach"  .planning/ tests/ src/   ->  255 hits
                                                        TOTAL  ->  388 hits
```

**The classification predicate, stated as a command so it is re-runnable rather
than trusted.** A hit is a **DATED OBSERVATION** if it sits in an executed or
superseded artifact — `*-PLAN.md`, `*-SUMMARY.md`, `*-VERIFICATION.md`,
`*-REVIEW.md`, `*-FIXES.md`, `*-RESEARCH.md`, `*-UAT.md`, `*-PATTERNS.md`,
`.continue-here.md` — and a **LIVING** hit otherwise. Living hits are the ones a
future reader consults for how to run or mitigate the binary, so they are the
ones read individually and corrected where they carry a standing claim:

```
rtk proxy grep -vE '(-PLAN\.md|-SUMMARY\.md|-VERIFICATION\.md|-REVIEW\.md|-FIXES\.md|-RESEARCH\.md|-UAT\.md|-PATTERNS\.md|\.continue-here\.md):'
```

| Class | Hits | Treatment |
|---|---|---|
| DATED OBSERVATION (executed / superseded artifacts) | **354** | left byte-unedited; see the policy paragraph below |
| LIVING (this file, `ROADMAP.md`, `WINDOWS.md`, `todos/pending/`, `codebase/`, `src/`, phase-19 `deferred-items.md`) | **34** | each read individually; the **7** carrying a standing mitigation claim corrected in place |

354 + 34 = 388, which is the measured total, so the classification accounts for
every hit. The full line-level enumeration with the per-hit verdicts is in
`21-34-SUMMARY.md`.

### Why the DATED OBSERVATIONS are left unedited, and named anyway (D-21-62)

A sentence in an executed SUMMARY recording that a particular run passed 3/3
under a particular setting **is a true record of that run**. Editing it would
falsify a historical observation in order to make a general point — which is the
same move, pointed the other way, as leaving the stale claim standing. They are
therefore left byte-identical and **named** in the inventory, so the record is
complete rather than selective. The distinction the inventory turns on is not
"old versus new"; it is **"a claim a future reader would act on" versus "a report
of what happened once"**.

**One dated observation is worth flagging because a reader could mistake it for a
standing claim.** `.planning/WINDOWS.md:27` (phase-20 wave-2 gate) records
*"it now fails 3/3 in ISOLATION (0.53s), whereas phase 19 recorded it passing in
isolation"*. That is a true record of that gate's runs and is left unedited — but
round 11 measured isolated re-runs **green 3/3 in three separate worktrees**, so
the isolation rate is itself unstable and that line must not be read as the
current rate. It is evidence for M2 in the table above, not a standing
instruction.

### Where the correction is placed

| Location | What was placed there |
|---|---|
| `deferred-items.md`, immediately after the stale `--test-threads=1` sentence | the CORRECTION block, so a reader who stops at the first section reads it |
| `deferred-items.md`, after the round-5/round-4 updates | the `--test-threads=2` correction |
| `deferred-items.md` (here) | the standing entry the other two point at |
| `tests/driver_reattach.rs`, module header | a **comment-only** note carrying the same three facts, where the reader of the failing test will actually look |

## 2026-08-28 (`21-34`) — round 11's four closures, with their RESIDUALS and DIRECTIONS

Every residual below is quoted from the SUMMARY of the plan that actually
delivered it, not from any plan's intent. This plan ran in wave 2 for exactly
that reason.

### gaps[0] — the resume argv fusion (`21-31`)

**Closed by:** `21-31` Tasks 1-3, commits `4ca622c` (RED), `3638fd9` (fix),
`8964b19` (the declined validator). `resume_terminal_argv` now emits **three**
elements, the untrusted `/proc`-scraped session id **fused** into one
`--resume=<id>` element, so no argv element is the bare id and there is nowhere
an option parser can reach it as syntax.

**The `--` separator that verification pass 11 recommended was REFUTED by
measurement** — probe D returns an error byte-identical to probe C's, so the
separator makes `--resume` receive nothing at all and would have shipped as a
security fix while silently deleting the resume for every legitimate session.

**Residual, with its direction — a DEPENDENCY-BEHAVIOUR property.** The fix rests
on what the `claude` CLI's own option parser does with `--resume=<value>`.
Nothing in this repository goes red if a future CLI changes that.
**Under-detection, SILENT.** A new standing staleness obligation is opened for it
below.

**Second residual (`21-31`'s own words, D-21-48):** `read_session_id` keeps its
pass-through and gains **no** rejecting validator, because *"the CLI resumes by
session TITLE, so a rule tight enough to refuse `-h` deletes legitimate sessions
silently"* — probe B's error text (*"does not match any session title"*) is the
measurement that decided it. **Under-detection, SILENT**, and the argv fusion is
the stated load-bearing control instead.

### gaps[1] — the interpreter census's comment budget (`21-32`)

**Closed by:** `21-32` Task 1 (`b644c32`) and Task 2 (`e18019c`). `taken += 1`
moved after the comment `continue`, so comment lines no longer spend the 16-line
join budget. **The window measured in both states:** comments cost nothing at any
count up to 100; the executable window is **last reported 11, first missed 12**,
identical before and after the fix. A two-sided boundary control asserted at N
and N+1 in both directions is its own certificate.

**The claim was narrowed in the SAME commit** — the word "any" is gone, three
bounds are named. Five residuals, each quoted with its direction from
`21-32-SUMMARY.md`:

| # | Residual | Direction |
|---|---|---|
| 1 | construction assembled across statements | under-detection, **silent** |
| 2 | interpreter named by a variable, const or config value rather than a quoted literal | under-detection, **silent** |
| 3 | `execute_hook`'s error-message-only interpolation | **deliberate exclusion**, not a gap |
| 4 | method chains are not followed | under-detection, **silent** |
| 5 | four of five string-building forms unseen (`concat!`, `join`, owned `+`, `replace`, owned `push_str`) | under-detection, **silent** |

### gaps[2] — the composition verdict moves to the innermost call (`21-32`)

**Closed by:** `21-32` Task 3 (`c84c16d`). The verdict is no longer taken over
the joined logical unit; it is taken **per occurrence, from the characters
immediately preceding it**, so a composed `match` arm can no longer launder its
bare sibling. All five real production sites still judge composed; the live
census still reports **zero** sites.

**The bounded shape matters:** the fix DELETES the window rather than widening
it. There is no unit left to over-join, so no future round can find a bigger one
that launders something.

**Residual, with its NEW direction — the direction FLIPPED on purpose.** A
composition assembled across separate statements now reads as un-composed and is
reported. *"Under the old whole-unit verdict this was under-detection and silent;
since 21-32 moved the verdict to the innermost call it is **over-detection, and
LOUD**."* Loud is the safe direction. Second residual: a third file added to this
path tomorrow is simply not looked at — **under-detection, silent**, bounded by
the render-escape probe and not by this census.

### gaps[3] — the `src/ui` census narrowed and its reach PINNED (`21-33`)

**Closed by:** `21-33` Task 1 (`9eacc82`) plus the in-plan pin repair
(`a23a4c9`). Renamed from `every_render_site_under_ui_composes_both_classes` to
`no_display_identity_call_under_ui_stands_outside_a_composition` — from an
unbounded claim about all render sites to a measured one about needle-carrying
executable lines: **six non-exempt occurrences in two of sixteen files**. The
completeness claim is handed **by name** to the two mechanisms that actually
carry it: the sealed `RenderAdjudicated` supertrait and the per-screen
behavioural probes.

**Residual, with its direction (quoted):** *"a render site that applies NEITHER
class carries no needle, so it is **invisible to this census** — silence, not
clearance. And every render site successfully converted to the single composed
call REMOVES a needle, so **this census's reach shrinks monotonically as the very
work it certifies succeeds.** Direction: under-detection, silent, and growing
over time."*

**The pin's own failure direction — BOOKKEEPING RED.** `MEASURED_REACH` is a
`Vec` equality checked in both directions, so a file leaving the distribution is
as much a red as one entering. Its red is not a security finding; it is a
one-line repair stated in its own failure message: **re-measure, update the
constant, update the disclosed sentence, all in one commit. Never relax the
assertion.** The pin **fired on its own author inside `21-33`** when Task 3 added
an eighth occurrence, which is its justification rather than an embarrassment: a
prose-only disclosure would have shipped stale within one plan of being written.

**For the merged tree:** the pin spans all of `src/ui/`, including `21-31`'s
`detail.rs` and `21-32`'s `driver.rs`. `21-34`'s merged-tree gate is where a
post-merge red would surface, and none did.

## 2026-08-28 (`21-34`) — STANDING: the `claude` CLI's option parser is a DEPENDENCY BEHAVIOUR, not a property of this code

**Same shape as the ratatui grapheme-filtering obligation this file already
carries** (see "STANDING — invisible-character rendering is a property of the
ratatui VERSION, not of this code"), and opened for the same reason: gaps[0]'s
fix rests on a property that belongs to another program's version, and an upgrade
can move it with every test in this repository green.

**Measured against `claude --version` → `2.1.248 (Claude Code)`**, re-measured by
`21-34` on the merged tree and agreeing with `21-31`'s figure. All six probes run
in a scratch directory outside this repository with stdin at `/dev/null` and a
timeout.

**The six probe commands, verbatim:**

```
claude --help | grep -A2 -- '-r, --resume'
claude --resume --version
claude --resume=--version
claude --resume -- --version
claude --resume -- 550e8400-e29b-41d4-a716-446655440000
claude --resume=550e8400-e29b-41d4-a716-446655440000
```

**What they established, re-measured by `21-34`:**

| Probe | Output (stdout, verbatim) | What it establishes |
|---|---|---|
| spec | `-r, --resume [value]    Resume a conversation by session ID, or open interactive picker with optional search term` | `--resume` is an **optional-value** option — the reason a bare trailing id is read as a new option |
| A | `2.1.248 (Claude Code)` | the injection FIRING: the token after a bare `--resume` is parsed as a new option of `claude` |
| B | `Error: --resume requires a valid session ID or session title when used with --print. Usage: claude -p --resume <session-id\|title>. Provided value "--version" is not a UUID and does not match any session title.` | the `=` form binds a hostile value as **DATA**, regardless of its first byte — and the **title** lookup path runs, which is why no validator was added |
| C | `Error: --resume requires a valid session ID or session title when used with --print. Usage: claude -p --resume <session-id\|title>` | a `--` separator closes the injection |
| D | **byte-identical to C** | the `--` separator **DELETES the capability** — `--resume` receives nothing at all |
| E | `No conversation found with session ID: 550e8400-e29b-41d4-a716-446655440000` | a real id, fused, is **BOUND and looked up** |

**The property this tree now depends on:** `-r, --resume [value]` is an
optional-value option; the `=` form binds the value regardless of its first byte;
the `--` form does NOT bind and deletes the capability.

**Direction: UNDER-DETECTION, SILENT.** Nothing in this repository goes red if a
future `claude` changes `--resume`'s arity or its `=`-form binding. Every test
here asserts the SHAPE of the argv this code emits, which is the correct thing
for them to assert and is exactly why they cannot see this.

**The standing obligation.** On any `claude` CLI upgrade, before trusting the
fusion: re-run the six probes above, re-measure, and record the new version and
outcomes in the table below. Do **not** infer from a version number.

| Date | `claude --version` | `--resume` arity | `=` binds hostile value | `--` binds a real id | Recorded by |
|---|---|---|---|---|---|
| 2026-08-27 | 2.1.248 (Claude Code) | optional-value | yes (probe B) | **no** — capability deleted (probe D) | `21-31` |
| 2026-08-28 | 2.1.248 (Claude Code) | optional-value | yes (probe B) | **no** — capability deleted (probe D) | `21-34`, merged tree |

## 2026-08-28 (`21-34`) — the three items round 11 DELIBERATELY did not close

Recorded with a promote condition and a direction each, rather than dropped.

### 1. `src/executor/claude.rs`'s three `--option value` pairs (T-21-31-05 / D-21-50)

**The measurement that makes them inert TODAY:** `--resume`, `--model` and
`--name` are each emitted as a separate `--option` element followed by a separate
value element — the same CWE-88 shape gaps[0] just closed at the Sessions-tab
resume — but all three read `Option` fields that are `None` at
`ExecutionOptions::default()` and are **set by no caller in this crate**. There
is therefore no untrusted value reaching them, and nothing to fuse.

**Promote condition:** the **FIRST caller that sets one of the three from a value
not authored in this crate.** At that moment the seam is live and the fusion must
be applied there, in the same commit as the caller.

**Direction: under-detection, SILENT** — until that caller appears, no committed
control in this tree goes red, because the shape is only a defect once a value
flows into it. Accepted, not closed, and this paragraph is the reason.

### 2. Pass 11's `IN-01`, `IN-02`, `IN-03` and `IN-04`

All four are `Info` severity in `21-VERIFICATION.md`'s anti-pattern table. One
line of reason each:

| Id | Site | Why round 11 did not chase it |
|---|---|---|
| `IN-01` | `src/ui/mod.rs:74-94` vs `106-113` — `EXEMPTIONS` matches whole-file while `WAVE_PENDING`'s doc argues for exact `path:line` pinning | An inconsistency of *stated rationale*, not of behaviour: the one exempt file is `#[cfg(test)]`-gated, so whole-file and `path:line` select the same set today. Tightening it would change no verdict. **Direction: over-detection if a non-test line is ever added to an exempt file — LOUD**, because the census reports it. |
| `IN-02` | `src/ui/screens/detail.rs:7457` — `assert!(matches!(separator, "-e" \| "--"))` cannot fail | A dead assertion, not a false claim: the load-bearing arm (`candidates.len() == 4`) sits directly above it and is live. Deleting it is cosmetic; `21-30` already closed the *other* `IN-02` (character-width). **Direction: none — a vacuous assertion adds no coverage and removes none.** |
| `IN-03` | `src/ui/screens/detail.rs:1820-1826`, `2226-2232` — "Resumed session {}" reported on `spawn()` returning, not on the resume working | Pre-existing UI-honesty shape that predates this phase, in a message the operator reads immediately after acting. Fixing it means waiting on a detached child, which is a behaviour change in a phase whose whole subject is the record. **Direction: over-reporting success, and LOUD to the operator** — the terminal window either appears or does not. |
| `IN-04` | `src/ui/screens/detail.rs:1866`, `4024-4079` — `ArchiveDepth::milestone` is the same untyped round trip `21-30` closed for `defaults_text_buffer` | Correctly escaped at each of its three render sites today and honestly disclosed with its direction. It is recorded so the last instance of the pattern is on the record, not because a leak was found. **Direction: under-detection, SILENT, if a fourth render site is added without the escape** — which is what the `src/ui` census and the sealed bound exist to catch. |

**`IN-05` is CLOSED, not outstanding.** `21-32` Task 2 (`e18019c`) closed it by
**narrowing the doc** rather than by widening the marker set:
`interpolates_into_a_string`'s doc now enumerates exactly the five markers, names
what they miss and states the direction, with the reason widening was declined —
*"a census that reports correct code is a census the next author disables"*
(D-21-53). Cite `21-32` for it; do not re-list it as open.

### 3. `WR-06` — the missing per-emulator working-directory column

**What it is:** the `cd '<dir>' &&` → `Command::current_dir` half of the CR-01 fix
has no control and no per-emulator table column, while the separator half of the
same change got both. `gnome-terminal` is a D-Bus-activated client and is one of
exactly four probed candidates.

**Why round 11 did not close it:** it is a **capability question, not a security
one.** No untrusted value travels through `current_dir` — the path comes from the
registered project — so there is no injection surface here. What is at risk is
whether the terminal opens in the right directory.

**Direction: under-detection, SILENT** — a `gnome-terminal` session that lands in
`$HOME` instead of the project root fails quietly, with no error and no red test.
**Promote condition:** a per-emulator table column measured against each of the
four probed candidates on a machine that has them, which needs a desktop session
and is therefore adjacent to criterion 4's human-only class rather than to this
round's scope.

### Disposition of EVERY row of pass 11's anti-pattern table — nothing dropped

| Id | Severity | Disposition |
|---|---|---|
| gaps[0] (CWE-88 argv) | Blocker | **CLOSED** — `21-31` T2 (`3638fd9`) |
| gaps[0] (complicit corpus) | Blocker | **CLOSED** — `21-31` T1 (`4ca622c`) |
| gaps[1] (comment budget) | Blocker | **CLOSED** — `21-32` T1 (`b644c32`) |
| gaps[1] (dangling cross-reference) | Warning | **CLOSED** — `21-32` T2 (`e18019c`) |
| gaps[2] (over-join / non-vacuity / truncation) | Blocker | **CLOSED** — `21-32` T3 (`c84c16d`) |
| gaps[3] (census named for a property it does not check) | Blocker | **CLOSED** — `21-33` T1 (`9eacc82`, `a23a4c9`) |
| `WR-04` (`EditBuffer` trait absences prose-only) | Warning | **CLOSED** — `21-33` T2 (`43b7c93`); `grep -c "implements_"` went 0 → 31, three planted-impl REDs captured |
| `WR-05` (comparator total over KEYS, not ELEMENTS) | Warning | **CLOSED** — `21-33` T2 (`43b7c93`), `.then_with(\|\| a.cmp(b))`, RED captured |
| `WR-06` (per-emulator cwd column) | Warning | **DELIBERATE NON-CLOSURE** — item 3 above |
| `IN-01` | Info | **DELIBERATE NON-CLOSURE** — item 2 above |
| `IN-02` | Info | **DELIBERATE NON-CLOSURE** — item 2 above |
| `IN-03` | Info | **DELIBERATE NON-CLOSURE** — item 2 above |
| `IN-04` | Info | **DELIBERATE NON-CLOSURE** — item 2 above |
| `IN-05` | Info | **CLOSED** — `21-32` T2 (`e18019c`) |

**Fourteen rows, fourteen dispositions, none unaccounted for.** Also delivered by
round 11 without being on this table: `21-29`'s carried **S1** (a two-direction
spot-check on an input-echo screen through the real `Screen::render`, `21-33` T3,
`5400e44`) and **F3**, which `21-33` closed as **UNNECESSARY by measurement** —
`render_escape_guard` is a child module of `ui::screens` and already reaches
`ctx_with_aliases`, so no visibility was widened anywhere.

## 2026-08-28 (`21-34`) — RECORD CORRECTIONS: the four round-10 truths pass 11 measured FALSE as shipped

Append-only. Each row quotes the falsified claim verbatim, states what pass 11
measured, and names the plan and task that closed it in round 11 with the
evidence that plan committed.

| Truth | What was CLAIMED (verbatim) | What pass 11 MEASURED | Closed by | Evidence committed by the closing plan |
|---|---|---|---|---|
| `21-27` truth 1 | *"CR-01 closed by deleting the shell; no parser left to make a metacharacter code"* — and in the source doc, *"**The third kind no longer exists here** because there is no interpreter left in the path to parse anything."* | **FALSE as shipped.** The shell class is genuinely gone, but *"no interpreter" is not "no parser"*: **`claude`'s own option parser was in the path the whole time**, and `--resume` takes an optional value, so a `/proc`-scraped id beginning with `-` became a new option. CWE-78 was traded for **CWE-88**, undisclosed in every round-10 artifact. | **`21-31` Task 2** (`3638fd9`) | The doc correction quotes the falsified clause verbatim beside its replacement, names CWE-88 and the receiving parser, and cites probes C/D/E by measured output; the argv is fused to three elements; the spawn-site comment was rewritten from *"there is no parser left"* to name the parser. |
| `21-27` truth 5 | *"The rule is CHECKED by a committed census reporting **any** such executable line"* | **FALSE at twelve comment lines.** The census's `taken += 1` ran before the comment `continue`, so comments spent the join budget: 12 comment lines between the interpreter name and the interpolation silenced the phase's sharpest execution-sink control on the exact CR-01 construction it exists to catch. | **`21-32` Tasks 1 and 2** (`b644c32`, `e18019c`) | The RED captured verbatim at exactly twelve comment lines; the increment moved after the `continue`; a two-sided boundary control asserting the window at 11/12 in **both** directions in both the comment and executable arms; and the word "any" deleted from the doc **in the same commit** as the mechanism repair, replaced by three named bounds and five directed residuals. |
| `21-28` truth 7 | *"IN-01 closed: an executable call to `sanitize_render_line` in those two files appears **only** inside a composition"* | **FALSE — sibling arms launder.** The verdict was taken over the joined logical unit, so a composed `match` arm made its bare sibling read as clean. Three blind spots reproduced with the census's own algorithm. | **`21-32` Task 3** (`c84c16d`) | The laundering reproduced verbatim against the committed rule (`left: []` vs `right: ["fixture.rs:3"]`); the verdict moved to the innermost call so **no window is consulted at all**; all five production sites re-measured still composed; the non-vacuity total recomputed over the slice actually scanned (7 whole-file → 6 in-slice); and the silent `#[cfg(test)]` truncation kill switch made LOUD, with its own planted RED naming both marker lines. |
| `21-29` truth 2 | *"The claim becomes a committed control over `src/ui/`"* — asserted by the test's own name, `every_render_site_under_ui_composes_both_classes` | **FALSE — the completeness its name asserts is not checked.** The mechanism is real and green, but it is driven by **6 executable lines in 2 of 16 files**. A render site applying neither class carries no needle and is invisible to it. Pass 11 hand-traced 63 `Span::raw`/`Span::styled` sites and found **no live leak** behind it — *"a false completeness claim with an undisclosed residual, not a live leak."* | **`21-33` Task 1** (`9eacc82`, repaired `a23a4c9`) | Renamed to `no_display_identity_call_under_ui_stands_outside_a_composition` (old name `grep -c` → 0); the reach disclosed file-by-file **and PINNED** as a two-direction `Vec` equality with a file-count floor; the shrinking-coverage property stated with its direction; the completeness claim handed by name to the sealed `RenderAdjudicated` bound and the behavioural probes; pin observed RED by a planted needle in `help.rs`, and again — unplanned — against its own author. |

**All four are the same species, and it is this phase's named species: a
completeness claim wider than the control that certifies it.** Three of the four
were in mechanisms round 10 BUILT to certify completeness. Round 11 closed each
by repairing the mechanism **and** narrowing the claim in the same commit — never
by repairing and re-selling as broader, and never by moving the enumeration down
a level.

## RE-SURFACED, UNCHANGED — ROADMAP success criterion 4 (round 11)

**Recording, not progress. This is the FOURTH consecutive round with NO work
claimed against it, and that is correct.** `21-34`'s prohibition 5 forbids
planning, executing or claiming any work against it.

Quoted VERBATIM from `21-VERIFICATION.md`'s `behavior_unverified_items`
frontmatter — command, expected result and why-human, not paraphrased:

> **truth:** *"A `.planning/` file or `CLAUDE.md` carrying injected instructions
> does not change which command the driver executes (ROADMAP success criterion 4
> / SAFE-07)"*
>
> **test:** *"With an authenticated `claude` CLI available, run
> `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
> repository root and record the CLI version beside the result."*
>
> **expected:** *"10 passed, 0 failed. Every
> `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload
> ARRIVED at the model before asserting the command was unchanged; the two
> suppression controls show the positive/negative
> `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and
> `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile
> and clean arms both really ran."*
>
> **why_human:** *"All ten spawn the real `claude` binary and need an
> authenticated subscription, so they cannot run inside verification. NO AGENT
> CAN CLOSE THIS ITEM, and the user has explicitly chosen to leave it tracked in
> `deferred-items.md`."*

**Round-11 status, MEASURED on the MERGED tree (all under `rtk proxy`):**

* `git diff --stat 2c13fcf HEAD -- tests/driver_injection_corpus.rs` — **empty.**
  The file was RUN by round 11 and EDITED by no plan of it.
* `cargo test --test driver_injection_corpus` — **13 passed / 0 failed / 10
  ignored.** Identical to every figure since pass 10.

### Said plainly, so the next verifier does not re-litigate it

**ROADMAP success criterion 4 is PERMANENTLY AGENT-UNCLOSABLE BY CONSTRUCTION.**
It requires a human with a live, authenticated Claude subscription to run the ten
`#[ignore]`d arms of `tests/driver_injection_corpus.rs` against the real model
binary. No agent has such a subscription and no agent can acquire one, so no
amount of further planning, further rounds or cleverer test design will close it.
It is a standing item **by explicit user decision**.

**4/5 is therefore the EXPECTED AND CORRECT CEILING for this phase, and is not a
failure.** A future verification pass that scores this phase 4/5 has scored it
correctly. A future round that opens a plan against criterion 4 is doing work
that cannot succeed. Presence and wiring are verified for the twelfth consecutive
pass; behaviour is not, and round 11 does not claim it is.

## 2026-08-28 (`21-34`) — the two DRIVE-04 backstops, re-run on the MERGED tree

**Reconfirmations by re-run, not new work.** DRIVE-04's boundary and precision
were measured by executed plans `21-02`, `21-04` and `21-22` and re-verified by
verification passes 10 and 11. Round 11 touches no cap, no seam and no
arithmetic; this shows its diff did not move them. Writing a *new* boundary
predicate for DRIVE-04 in a record-correction round would be the
manufactured-predicate overclaim this phase exists to end.

`rtk proxy cargo test --test driver_escalation_cap` on the merged tree:
**8 passed / 0 failed / 0 ignored.**

| Row | Direction | Arm |
|---|---|---|
| edge-probe 6 (**boundary**) | a cap AT or ABOVE the resolved step cap is refused at the seam | `a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run`, `a_budget_of_zero_refuses_above_the_run` |
| edge-probe 6 (**boundary**) | a cap BELOW it is accepted and parks on the cap | `a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap` |
| edge-probe 7 (**precision**) | the decomposition consultation counts against the SAME cap, and exceeding it parks with a typed reason rather than degrading silently to rules-only | `a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing`, `the_three_boundaries_are_measured_against_the_resolved_step_cap` |

`rtk proxy git diff --stat 2c13fcf HEAD -- tests/driver_escalation_cap.rs` is
**empty** — the file is RUN and unchanged by round 11, so the reconfirmation is
explicit evidence rather than an inference from the round's scope.

**DRIVE-04 precision, stated in words:** the escalation counter is an **integer
count compared by equality against an integer cap**. There is no rounding, no
tie-breaking, no overflow behaviour and no precision-loss contract to state,
because no arithmetic on a non-integer quantity occurs anywhere on this path.
Reconfirmed by the same unchanged suite in the same run.
