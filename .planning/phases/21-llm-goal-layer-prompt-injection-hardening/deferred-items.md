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
