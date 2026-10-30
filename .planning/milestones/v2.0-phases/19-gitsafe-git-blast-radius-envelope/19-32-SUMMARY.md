---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 32
subsystem: envelope
tags: [security, corpus, red-handoff, word-set, path-set, linearity]
status: complete
requires: [19-31]
provides:
  - "tests/envelope_word_set.rs — the TWELFTH evidence file, RED on ten derived rows"
  - "tests/envelope_wrapper_class.rs — the CONTROL-CARRIER axis at FOURTEEN classes"
  - "the RESTATED THREE-AXIS residue condition"
  - "both design answers, derived rather than chosen"
  - "an exact handoff number and a two-anchor handoff commit for 19-33"
affects: [19-33]
tech-stack:
  added: []
  patterns:
    - "a corpus written and committed RED before the rule exists — the thirteenth round"
    - "a naming contract enforced as a GATE: derived-post-fix rows carry a marker and nothing else does"
    - "a SEVERANCE-RETIRABLE row isolated so a severance has an exactly-one-name failure set"
key-files:
  created:
    - tests/envelope_word_set.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "T-19-122's boundary is a SECOND `Segment` field and a THIRD word class, DERIVED from the fenced-file enumeration: widening `pathname_target` turns two fenced pins red and the second field turns neither"
  - "T-19-123's boundary is an ANCESTOR clause BESIDE the existing prefix, bounded at `<root>`, growing the protected set by EXACTLY ONE path — not the prefix widened to the root"
  - "T-19-124's property is LINEARITY pinned as a RATIO, not a wall-clock speed"
metrics:
  duration: one session
  completed: 2026-09-04
actuals:
  tokens: 214000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 32: The Word Set and the Path Set — Summary

**The other two ways a boundary can be silent: the words the reader never
receives, and the paths the comparison never protects — measured, drawn into the
corpus, and committed RED.**

---

## FIRST: this plan closes nothing, and `/gsd-secure-phase 19` is NOT cleared

**Not by this plan, not by `19-33`, and not by the two together.** Eight threats
remain open at `high`:

`T-19-86` (by explicit user scoping decision), `T-19-91`, `T-19-111`,
`T-19-112`, `T-19-116`, `T-19-121`, **`T-19-122`** and **`T-19-123`**.

`T-19-124` is open at `medium`; `T-19-125` and `T-19-126` at `low`.

**This plan measures, widens the corpus and goes RED. It closes nothing at all**,
and whether `19-33`'s rules close anything is **audit 13's judgement rather than
either plan's claim**. Audit 12's finish-line sentence — whether the open set has
become the human-decision items plus the deliberately-unruled residues — is
**not written here and is not `19-33`'s to write either.**

**No unqualified "T-19-60 is closed" appears anywhere. Only the WRAPPER-OPERAND
sub-class of `T-19-60` is closed.**

---

## The base, verified before the first measurement

```text
git diff --numstat f06d153..8659a18 -- src/ tests/   ->  EMPTY
git diff --numstat 8659a18..4e77a0e -- src/ tests/   ->  EMPTY
```

So this plan's base is the tree audit 12 measured against, and the two
plan-check commits between `8659a18` and `4e77a0e` touched only `.planning/`.

**The `dd17bfb` control binary was built OUTSIDE the repository** —
`git archive dd17bfb | tar -x -C /tmp/…/dd17bfb-control` followed by
`cargo build` there. **The working tree was never checked out, reset or
stashed**, and `git status --short` showed only the untracked `.gsd/` throughout.

---

## The three commits, in order

| # | SHA | What | `src/` hunks | `tests/` deletions |
|---|---|---|---|---|
| 1 | `172b1d3` | `tests/envelope_word_set.rs` — the TWELFTH evidence file | **0** | **0** (+3339 / −0) |
| 2 | `604c644` | `tests/envelope_wrapper_class.rs` — the axis at fourteen classes | **0** | **8** (+1222 / −8) — see the finding below |
| 3 | *this commit — resolve it by either anchor below* | the record, `deferred-items.md` and this SUMMARY | **0** | **0** |

`git diff --numstat HEAD~3..HEAD -- src/` is **EMPTY**.
`src/envelope/advisory.rs` shows **zero diff lines**.
Neither `Cargo.toml` nor `Cargo.lock` moved (`T-19-SC`).

---

## THE HANDOFF ANCHOR — two anchors, required to AGREE

`19-33`'s final gate must resolve this plan's last commit, and every earlier
round resolved it from a commit-message SHAPE no plan ever mandated. **A gate
that depends on a message shape is a gate one reworded commit disarms.** So two
anchors are established and `19-33` requires them to agree:

* **the record commit's message begins exactly `docs(19-32)`** — mandated, not
  assumed;
* **the record commit is the ONE that adds `19-32-SUMMARY.md`**, so
  `git log --format=%H -n 1 -- .planning/phases/19-gitsafe-git-blast-radius-envelope/19-32-SUMMARY.md`
  resolves it from FILE CONTENT rather than from a message.

**Both were verified to resolve to the same SHA by this plan's own gate**, which
asserts the file-content anchor equals `git rev-parse HEAD` and that the subject
line matches `^docs(19-32)`.

**The literal SHA is deliberately NOT written here, and that is the point rather
than an omission.** A commit's own hash cannot be embedded in a file that commit
creates — writing it would require an amend, which changes the hash, which
falsifies the line. **That circularity is exactly why the anchors are RESOLVERS
rather than a transcribed number**, and `19-33` resolves them itself:

```bash
BASE_BY_CONTENT=$(git log --format=%H -n 1 -- \
  .planning/phases/19-gitsafe-git-blast-radius-envelope/19-32-SUMMARY.md)
BASE_BY_MESSAGE=$(git log --format=%H -n 1 --grep='^docs(19-32)')
test "$BASE_BY_CONTENT" = "$BASE_BY_MESSAGE"   # 19-33 REQUIRES them to AGREE
```

**If they disagree, `19-33` must stop and report it** — a disagreement means
either the record commit was reworded or a later commit claimed the prefix, and
resolving the base from a message shape alone is the failure this contract
exists to prevent.

### AND THE MESSAGE ANCHOR IS **NOT UNIQUE**. Measured, and it is the whole reason the second anchor exists.

```text
git log --format='%h %s' --grep='^docs(19-32)'
  2789395  docs(19-32): the three-axis condition, both design answers, and the RED handoff
  4e77a0e  docs(19-32): plan-check — the seam again, ninth time and self-inflicted
```

**TWO commits in this phase begin `docs(19-32)`** — this record and the
plan-check that preceded it. `git log -n 1 --grep` resolves to the NEWEST, which
is correct today **and only by ordering**. A message-shape anchor was never a
contract; it was a convention that happened to hold, and here it is one commit
away from resolving to a plan-check that contains no corpus at all.

**So `19-33` MUST resolve its base by FILE CONTENT and use the message only as
the cross-check**, exactly as the snippet above does. A gate that took
`--grep` alone would have silently diffed against the wrong base.

---

## SEVERANCE-RETIRABLE ROW

```text
after_19_33_the_candidate_scan_is_linear_in_the_words_length
```

**It carries the linearity ratio assertion and NOTHING ELSE.** The no-slash
control, the curve record and every other `T-19-124` row live in separate `#[test]`
fns, **so that under a severance of `19-33`'s work-bound task the failure set is
EXACTLY this one name.**

**`19-33`'s two gates must each carry an explicit branch tolerating exactly that
name and no other.** Every other `after_19_33_*` row and the one `_after_19_33`
axis row must go GREEN.

This is the seam that halted `19-23` and needed a human authorisation twice
since. It earns a named contract rather than a precedent.

---

## THE COMPLETE RED NAME LIST — `19-33`'s handoff contract

**Eleven rows, all carrying their marker. `19-33`'s first action is confirming
this list.**

In `tests/envelope_word_set.rs` (prefix `after_19_33_`), ten:

```text
after_19_33_a_here_string_word_naming_the_envelope_directory_is_refused
after_19_33_a_here_string_word_naming_the_guards_own_binary_is_refused
after_19_33_a_here_string_word_carrying_the_path_at_a_non_zero_index_is_refused
after_19_33_a_non_pathname_operator_other_than_a_here_string_reaches_the_same_way
after_19_33_a_word_naming_the_ancestor_of_the_envelope_directory_is_refused
after_19_33_the_ancestor_spellings_that_are_not_rm_are_refused_the_same_way
after_19_33_the_two_segment_composite_naming_the_ancestor_is_refused
after_19_33_ordering_pin_a_the_here_string_carrier_wins_its_own_segment
after_19_33_ordering_pin_b_the_ancestor_wins_its_own_segment
after_19_33_the_candidate_scan_is_linear_in_the_words_length      <- SEVERANCE-RETIRABLE
```

In `tests/envelope_wrapper_class.rs` (suffix `_after_19_33`), one:

```text
a_command_naming_a_carrier_in_a_word_class_or_a_path_set_rule_a_has_no_clause_for_is_refused_after_19_33
```

### THE COMPLETE PERMITTED LIST — the rows that MUST NOT MOVE

Asserted exit 0 (or at their unchanged identifier) BEFORE **and** AFTER:

```text
cat <<<x  /  cat <<< /tmp/x            class 13 is a PATH class, not a here-string ban
echo x >&/tmp/plain-outside            the `>&` half's own cost control
xargs rm -rf <<< of=/tmp/g/x           the non-zero-index half's cost control
git <<<x push --force origin main      exit 2 force_push_blocked — T-19-97 / round 6
xargs rm -rf <<EOF\n/tmp/plain…\nEOF   the heredoc CONTROL beside the corrected row
ls <ENV>/unrelated-sibling             the ANCESTOR-vs-PREFIX discriminator
rm -f <ENV>/unrelated-sibling          the same as a WRITE
ls <ENV>/beta  /  rm -rf <ENV>/beta    a second alias directory under the same root
ls <ENV-PARENT>  /  ls /tmp  /  ls /   the ancestor STOP; no rule above the root
df /  /  ls /home                      the same
<user-set root>/someone-elses-file     reachable CONFIGURATION, not a thought experiment
ls <BINPAR>                            the binary's EXACT-PATH control
cp /bin/true <BINPAR>/some-other-file  the same, as a write
git --git-dir=/tmp/g status            pinned PERMITTED in FOUR other files
dd of=/tmp/g/./x                       T-19-125's outside-the-path-set control
a 100 000-character word, NO slashes   exit 0 in 29 ms — the DECISIVE cost control
a symlinked-to-REGULAR ledger          exit 0 AND COUNTED
a FRESH root with no ledger            exit 0, one line
git -c alias.p='!git push --force origin main' p    T-19-86, may not move
git -c alias.q='!git -c credential.helper=store credential fill' q   T-19-86's route
every round 4-12 mechanism pin         listed in full in the 19-SECURITY.md record
```

---

## The naming contract — a GATE rather than a habit, and it CAUGHT things

**Every `#[test]` asserted at a DERIVED POST-FIX verdict is named `after_19_33_*`
in the evidence file and suffixed `_after_19_33` on the axis; nothing else
carries the marker.** All three RED gates extract the failing names from the log
and require **every** failure to carry one.

**It fired three times, and every firing was a real defect this plan had
written:**

1. **A twin pair driven against the WRONG envelope root.** The bare-remote
   fixture asked the guard about `<fixture>/env/alpha` while handing it a
   different `GSD_MM_ENVELOPE_ROOT`, so the "control" measured a silence rather
   than a refusal and answered exit 0. `word_is_within` compares against the
   directory THE GUARD WAS HANDED; a pair driven against a different root
   measures two silences rather than one refusal and one permit.
2. **A credential fixture that never invoked the alias.** The helper appended
   `credential fill` to every argv, so the alias-body row drove
   `git -c alias.q=… credential fill` — which never runs the alias at all and
   **measured the CONTROL twice**. That is a mis-quoted argv silently changing
   the class, exactly what this plan's own discipline says to verify by eye.
3. **A mis-derived class count** on the axis — 15 against a derived 13 — which
   surfaced TWO further predicate collisions (see below).

**A bare "something failed" check would have passed on all three.** At the end
no failure lacked its marker except one documented flake, recorded below.

---

## The suite numbers, with the arithmetic STATED and CHECKED

```text
rtk proxy cargo test --no-fail-fast
  48 result lines, 13 ignored
  run A: 1902 passed + 12 failed = 1914   (a documented flake fired)
  run B: 1903 passed + 11 failed = 1914   (it did not; see below)

baseline, recorded by 19-31-SUMMARY.md and re-measured by audit 12      1871
new #[test] fns, counted from `git show` over both code commits:
    tests/envelope_word_set.rs                                            38
    tests/envelope_wrapper_class.rs                                        5
                                                                    -------
                                                                          43
  1871 + 43 = 1914      <-  EXACT, no disagreement

cargo build                            exit 0
cargo clippy -- -D warnings            exit 0   (`--tests` is NOT the gate)
```

A red test RAN, so red→green leaves the total unchanged and every increase comes
ONLY from new `#[test]` fns. **The identity holds exactly.**

### All NINETEEN `envelope_*` binaries RAN

```text
envelope_advisory            10/0     envelope_literal_decision    43/0
envelope_argv_deletion       20/0     envelope_pr_cap              11/0
envelope_callee_grammar      19/0     envelope_reparsed_value      34/0
envelope_carrier_reach       39/0     envelope_tracer               6/0
envelope_command_position    18/0     envelope_wiring              14/0
envelope_config_resolution   30/0     envelope_word_set            28/10   <- NEW
envelope_control_carrier     37/0     envelope_wrapper_bypass      13/0
envelope_credential           6/0     envelope_wrapper_class       59/1
envelope_expansion_slots     32/0     envelope_interior_path       41/0
envelope_hook_refusals        7/0
```

**There were EIGHTEEN before this plan. A run reporting eighteen would be a run
in which this plan's own evidence file did not execute.**
`envelope_config_resolution` is at **30/0** and
`every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule`
is GREEN and non-vacuous inside `envelope_interior_path` at 41/0 — `19-31`'s
file, which this plan did not open.

### A documented flake FIRED ON ONE RUN AND NOT ANOTHER. Recorded, not fixed.

**Three full observations, all recorded, because the point of a flake record is
the observation and not a tidy conclusion:**

* **Full-suite run A** — 1902 passed / **12** failed. The twelfth was the flake
  below, **the ONE failure lacking a marker**, and this plan's own gate flagged
  it as such.
* **An isolated re-run of `tests/driver_reattach.rs`**, driven once as a
  MEASUREMENT rather than a fix — **1 passed / 2 failed**, so BOTH documented
  flakes of that file fired.
* **Full-suite run B**, after the record commit — 1903 passed / **11** failed.
  **The flake did not fire at all**, and every failure carried its marker.

`passed + failed` is **1914 in both full-suite runs**, so the handoff number is
unaffected by which way the flake lands.

```text
---- a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step stdout ----
thread 'a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step'
panicked at tests/driver_reattach.rs:450:5:
assertion `left == right` failed: exactly one project has a run to observe
  left: 0
 right: 1
```

**It is out of scope, not fixed, not worked around, and its relation to this
round is claimed in NEITHER direction.** Its firing in run A and its silence in
run B are both recorded; **neither is evidence about the other**, and **absence
is not evidence it is fixed.**

`envelope_tracer` answered 6/0 in both runs with no `ExecutableFileBusy` firing
and no ETXTBSY over the binary — **and absence is not evidence either of those is
fixed either**, which matters here because the here-string BINARY rows exercise
`C-10`'s own seam directly.

---

## THE RESTATED THREE-AXIS RESIDUE CONDITION

Audit 12's diagnosis is adopted whole: the condition at `policy.rs:5900-5905`
*"enumerates the silences of a READING and the code's silences are also in its
WORD SET and in its COMPARISON."* It names the word set (*"in either word
class"*) and the path set (*"against both paths"*) as **premises** and then says
nothing about either.

> **AXIS 1 — WHICH WORDS REACH THE READER.** This predicate reads every LITERAL
> word of the segment that is not an operator, every LITERAL PATHNAME redirection
> target, and every LITERAL NON-PATHNAME redirection target. **Its silence on this
> axis is a word the tokenizer produced for no class at all** — a production that
> failed to complete, which fails closed at `Token::redirection_unresolvable`
> rather than being read.
>
> **AXIS 2 — WHAT THE READER SEES IN A WORD IT RECEIVED.** It reads every
> `/`-anchored substring of that word's text, normalised. **Its silences are a
> word the SHELL MAY REWRITE**, because the guard cannot know its final text; **a
> word whose TEXT CARRIES NO ABSOLUTE PATH ANYWHERE**, because there is nothing in
> it to normalise; **and a word that reaches a protected path ONLY THROUGH A
> LINK**, because no lexical reading of the text names it.
>
> **AXIS 3 — WHICH PATHS THE COMPARISON PROTECTS.** A candidate under this run's
> envelope directory, a candidate that is a proper ANCESTOR of it at or under this
> run's envelope ROOT, and a candidate equal to this run's own binary. **Its
> silences are an ancestor ABOVE the root — `$HOME`, `/tmp`, `/` — and the
> binary's PARENT and every ancestor of it**, because refusing a word that names
> one of those is an outage rather than a boundary (AR-19-11).

**What the MEASUREMENT showed on each axis, rather than what the plan predicted:**

* **AXIS 1 is NARROWER than the plan assumed.** Of the five non-pathname
  operators, **two carry the harm and three do not**: `<<<` reaches in every
  spelling driven, `>&` reaches (and was not predicted to), while `<<`/`<<-` with
  the path as the DELIMITER and `<&` with a path reach nothing. The heredoc BODY
  form reaches and **is already refused**. The axis-1 residue — an incomplete
  production — genuinely fails closed.
* **AXIS 2 is unchanged** from round 12 and its three silences hold. `T-19-125`
  shows the DISCLOSED WORDING of the axis is one operation short, not the axis.
* **AXIS 3's growth is EXACTLY ONE PATH**, confirmed by construction:
  `ledger_path_in` refuses a non-plain-component alias, so the ancestor set at or
  under the root is `{<root>}`. The residue ABOVE the root is real and unruled.

**No count is written on any axis, and none of it is handed to a pin, a schedule
or a version witness.** `19-25`'s witness schedules a re-audit against a FUTURE
GIT; every one of these is reachable by the driven agent TODAY.

---

## THE `T-19-122` DESIGN ANSWER, derived from the fenced-file enumeration

| Design | What it does | What it turns RED |
|---|---|---|
| **(A)** widen `RedirectionOperator::pathname_target` | one flag flip | **`only_a_literal_pathname_target_reaches_the_segment_and_the_tokens_do_not_move`** (`policy.rs:10693-10727`) and **the twelve-operator grammar pin** (`:10586`, `:10603`) |
| **(B)** a SECOND `Segment` field + a THIRD word class at the SAME site | one field, one `else if`, one `.chain()` | **nothing** |

**Design (B) is MANDATED and (A) is FORBIDDEN, and the reason is a MEASURED FENCE
rather than a preference.** The **third design** — *treat every
deleted-and-unrecorded word as UNRESOLVABLE* — is **rejected in writing**: it
would deny `cat <<< hello`, `sort <<< "$x"` and every fd duplication on the line,
an OUTAGE rather than a boundary (AR-19-11).

**The principled restatement:** the predicate's subject is what a line NAMES, not
what the shell does with a word — so the word set is *every LITERAL word the line
contains, whether it survives into argv or is consumed by a redirection, pathname
or not.*

**Disclosed over-refusal, stated in advance:** `cat <<< <ledger>` becomes
refused — the same family as the already-disclosed `cat <ledger>`, failing CLOSED
and bounded to this run's own two paths. `cat <<<x` and `cat <<< /tmp/x` are
pinned permitted.

### THE WORD-SET SWEEP, with the `bash` reach column and the twin beside every row

```text
                                                 guard   TWIN    bash REACH
xargs rm -rf <<< <ENV>/alpha                     exit 0  exit 2  the directory was DELETED
xargs rm -f  <<< <ENV>/alpha/pr-ledger.ndjson    exit 0  exit 2  the ledger was DELETED
xargs cp /bin/true <<< <BINARY>                  exit 0  exit 2  C-10, layers 2 AND 3
xargs -I{} rm -rf {} <<< <ENV>/alpha             exit 0  exit 2  DELETED
xargs -n1 rm -rf <<< <ENV>/alpha/hooks           exit 0  exit 2  (a second `<<<` spelling)
xargs rm -rf <<< "x <ENV>/alpha"                 exit 0  exit 2  DELETED (xargs splits on space)
xargs rm -rf <<< of=<ENV>/alpha                  exit 0  exit 0  reach NOT claimed — a shape fence
echo evil >&<ENV>/alpha/pr-ledger.ndjson         exit 0  exit 2  TRUNCATED and rewritten
: >&<ENV>/alpha/pr-ledger.ndjson                 exit 0  exit 2  truncated to ZERO bytes

xargs rm -rf <<EOF\n<ENV>/alpha\nEOF             exit 2    —     DELETED  <- CORRECTION, below
xargs rm -rf <<<ENV>/alpha   (delimiter=path)    exit 0    —     REACHES NOTHING
xargs rm -rf <<-<ENV>/alpha  (delimiter=path)    exit 0    —     REACHES NOTHING
cat <&<ENV>/alpha/pr-ledger.ndjson               exit 0    —     `ambiguous redirect`

cat <<<x  /  cat <<< /tmp/x                      exit 0    —     the class's PATH controls
git <<<x push --force origin main                exit 2 force_push_blocked, unchanged
```

### THE HEREDOC BODY FORM — a CORRECTION to audit 12, written BESIDE its row

Audit 12 records that `xargs rm -rf <<EOF\n<ENV>/alpha\nEOF` deleted the
directory under real `bash` — a claim about the HARM, with **no guard verdict
beside it**. **Driven here, the guard answers exit 2
`envelope_assertion_failed`.** `\n` is in `SEPARATORS` (`policy.rs:2422`), so the
body's words are ordinary tokens of a SEGMENT OF THEIR OWN and round 10's clause
already reaches them.

**Both halves are true**: the harm audit 12 measured is real, and the verdict it
did not measure is a REFUSAL. It is now pinned exit 2 BEFORE AND AFTER with an
outside-the-root heredoc control beside it. **Audit 12's subsection is not
edited** — the correction is written beside it, which is the discipline audit 12
used for its own three.

### A ROUTE THE PLAN DID NOT PREDICT: `>&` WITH A PATH

Bash's `[n]>&word` duplicates an fd when `word` is digits — **but when it is a
PATH, bash treats the redirection as `&>word` and OPENS THE FILE FOR WRITING.**
Measured: a three-line 18-byte ledger became a 5-byte file holding `evil`, and
`: >&<ledger>` truncated it to ZERO bytes and reset a fired cap.

`redirection_operator` is CORRECT to answer `pathname_target = false` for `>&` —
the GRAMMAR says it takes an fd number — and bash's fallback is what makes it
reach a file anyway. **That is the second operator the class needed, with a
driven reach rather than a shape**, and it is what stops `19-33` writing a
here-string-only rule.

### The PROCESS finding, written BESIDE

Audit 11 recorded the five non-pathname operators as *"correctly NOT recorded as
carriers, so the round bought its rule without an over-refusal"*, with `cat <<<X`
as the witness. **Right about `cat`, wrong as a general statement: a here-string
is a DATA channel and `xargs` turns data into argv.** Same shape as `19-28`
seeing `T-19-119`'s spelling and calling it *"Not a defect"* — **a
recorded-and-misgeneralised observation is how a finding survives a round that
already saw it.** Neither subsection is edited.

---

## THE `T-19-123` DESIGN ANSWER — the path set, the growth, the stop, the cost

```text
a candidate is refused when it is
  (1) UNDER `<root>/<alias>`, component-wise             <- the existing PREFIX, unchanged
  (2) a proper ANCESTOR of `<root>/<alias>` that is
      itself AT OR UNDER `<root>`                        <- the NEW clause
  (3) EQUAL to `current_exe()`                           <- the existing EQUALITY, unchanged
```

**It grows by EXACTLY ONE path**, because the alias is a PLAIN SINGLE PATH
COMPONENT — `ledger_path_in` refuses anything else — so the ancestor set at or
under the root is `{<root>}`. The clause is written in its GENERAL form anyway,
so it stays correct if the directory ever becomes deeper.

**It STOPS at `<root>`, on OWNERSHIP grounds.** `<root>` is created by this tool
and holds only alias directories it created; above it the chain is
`~/.local/share`, `$HOME`, `/tmp`, `/` — shared with everything the user has. A
boundary reaching them would refuse `ls /`, `df /`, `du -sh $HOME` and `ls /tmp`:
**not a boundary but an OUTAGE** (AR-19-11). Same argument `word_is_exactly`
already gives one path over.

### The four-row ANCESTOR-vs-PREFIX discriminator

```text
                                    ancestor clause (mandated)   prefix widened to <root> (forbidden)
rm -rf <root>                       REFUSED                      REFUSED
rm -rf <root>/alpha                 REFUSED                      REFUSED
ls <root>/unrelated-sibling         PERMITTED                    REFUSED   <- the discriminator
GSD_MM_ENVELOPE_ROOT set by the
user to a shared directory;
ls <that>/anything-at-all           PERMITTED                    REFUSED   <- the whole subtree
```

**The fourth row is reachable CONFIGURATION and it was DRIVEN**: the guard was
run with the root pointed at a directory holding unrelated files, and both
`ls <that>/someone-elses-file` and `rm -rf <that>/someone-elses-dir` are exit 0.

### The residue ABOVE the root

An ancestor above `<root>` — `rm -rf /tmp` when the root is `/tmp/xyz` — reaches
the same nine carriers and **no rule is written for it.** Registered, disclosed,
UNACCEPTED, named on axis 3, and RECORDED in the corpus in NEITHER direction.

### BOTH FENCED PINS NAMED AS `19-33`'s CROSS-FENCE EXCEPTION

`policy.rs:10022-10026` and `tests/envelope_interior_path.rs:2341` both pin
`T-19-123` PERMITTED **by name**, and both are GREEN and NON-VACUOUS today.
Audit 12: *"the code does exactly what the pin says. What is missing is the
threat row and the residue clause, not the implementation."*

**`19-33` moves both as a NAMED, BOUNDED, CROSS-FENCE exception with each pin's
reasoning REWRITTEN rather than deleted** — the shape round 11 used for
`direction_i_…_stays_permitted`, which needed a human authorisation. **This plan
performs neither move.** The mechanical form is
`every_fenced_row_keeps_its_answer_under_the_widened_path_set_except_the_two_named_pins`,
which asserts the row `false` today and `true` under the simulated post-fix
design, and asserts thirteen other fenced unit pins and eighteen fenced corpus
rows move in NEITHER direction.

### The two END-TO-END drives, each with both remote SHAs and a fired-then-reset cap

Both `T-19-122` and `T-19-123` were driven through a REBUILT bare-remote fixture
— a bare upstream, a working clone on `refs/heads/gsd-auto/alpha/w`, and both
stubs delivered through the `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0`
triplet naming `core.hooksPath`:

```text
CONTROL  hooks present, git push --force  ->  REFUSED, remote main UNMOVED
GUARD    the permitted spelling           ->  exit 0, its twin at exit 2
LEG B    performed under real bash        ->  the WALK of the envelope found NOTHING
         same push                        ->  COMPLETED, remote main MOVED
CONTROL  stubs restored, remote rewound   ->  REFUSED, UNMOVED again
```

The SHAs are recorded by the tests themselves and asserted `assert_ne!` across
LEG B and `assert_eq!` across both controls, so the movement is **observed**
rather than described. Each cap reset was re-driven in **ONE persistent root**
from a FIRED cap: four `gh pr create` calls (1 permit, 3 `pr_cap_exceeded`, four
ledger lines), `rm -f <ledger>` at exit 2 as the control, the permitted spelling
at exit 0, performed, and call 5 **PERMITTED AGAIN with a one-line ledger.**

---

## `T-19-124` — the curve on three trees, and why the pin is a RATIO

```text
slashes        bytes        HEAD          dd17bfb
  1 000        2 000          68 ms         5 ms
  2 000        4 000         249 ms         5 ms
  5 000       10 000       1 524 ms         7 ms
  8 000       16 000       3 904 ms         8 ms
  9 000       18 000       4 957 ms         —      <- GUARD_TIMEOUT_SECS = 5 crossed
 10 000       20 000       6 099 ms         —         between 18 KB and 20 KB HERE
 20 000       40 000      24 425 ms        14 ms
 50 000      100 000     151 249 ms        28 ms
400 000      800 000   DID NOT ANSWER in 300 s (exit 124, hard external timeout)
100 000 chars, NO slashes    29 ms        21 ms   <- the DECISIVE control
```

**A CORRECTION TO THIS PLAN'S OWN EXPECTATION.** Audit 12 measured 5 873 ms at
8 000 slashes and concluded *"a 16 KB command line crosses `GUARD_TIMEOUT_SECS =
5`"*. On THIS machine 16 KB answers in 3 904 ms and the deadline is crossed
between **18 KB and 20 KB**. The mechanism is identical and the conclusion holds
one size up — **which is precisely why the corpus pins a RATIO and not a wall
clock.**

**`MAX_GUARD_REQUEST_BYTES = 1 MiB` bounds the INPUT and bounds no work** —
`T-19-120`'s own shape in the round that fixed `T-19-120`. Measured directly: a
1 MiB command is refused in 20 ms by that bound, **so the WORST CASE is a request
JUST UNDER it**, and 800 KB did not answer in five minutes.

**The byte floor is INSUFFICIENT, and the arithmetic is written out.** Candidate
`i` of a 16 KB word with 8 000 slashes has length `2 * (8000 - i)`; a floor of
about 18 bytes removes exactly those `i` with `2 * (8000 - i) < 18`, i.e.
`i > 7991` — **NINE candidates out of EIGHT THOUSAND.** The other 7 991 each
still cost O(word length). **A floor prunes the CHEAP END of a quadratic.**

**And a candidate CAP must be over a quantity the request cannot MULTIPLY**: a
per-WORD cap is amplified by the word count and a per-SEGMENT cap by the segment
count, so a 1 MiB request of many medium words each spending exactly the cap
defeats either.

**THE PROPERTY: LINEARITY, with a FAIL-CLOSED work ceiling beside it as the
backstop.** Pinned as `t(8 000)/t(2 000) < 8` — four times the input predicts
≈4× linear and ≈16× quadratic, so 8 leaves a factor of two of margin in both
directions. **Measured 15.7 out of band and 13.19 under full suite load.**
Process startup is a fixed additive term in both measurements and only ever moves
the ratio DOWN, so it cannot manufacture a red. The 50 000-slash and 800 KB rows
are RECORDED with their wall times and **not asserted** — a test that is RED for
hours is not a test.

**THE BEHAVIOURAL HALF is UNMEASURED and claimed in NEITHER direction.**

---

## `T-19-125` — the two normalising instances, with the absence verified mechanically

`dd of=<ENV>/./alpha/x` and `dd of=<ENV>/zzz/../alpha/x` are both **exit 2**, and
the corpus **asserts mechanically** that neither word contains `<ENV>/alpha` as a
substring at all — and that the simulation of today's rule answers `true` for
both anyway, because the comparison NORMALISES. Their non-normalising twin
`dd of=<ENV>/alpha/x` is exit 2 beside them and the outside-the-path-set control
`dd of=/tmp/g/./x` is exit 0.

**The BEHAVIOUR is asserted; both WORDINGS are RECORDED as `19-33`'s to repair
whether or not any rule lands** — the disclosed cost's *"as a `/`-anchored
substring"* (`policy.rs:6044-6048`) and `envelope_carrier_refusal`'s *"To
proceed: name a path outside that directory"* (`:6168`, `:6180`), which is STALE
for exactly this case.

---

## `T-19-126` — two REAL standing pins, GREEN before and after

**These are STANDING CONTROLS, not this round's RED, and that is stated so they
are not counted as evidence of a fix.**

* **BEHAVIOURAL:** a ledger symlinked to a REGULAR file, driven through the
  guard — **exit 0 AND the line COUNTED through the link** — beside FIFO,
  directory, symlink-to-character-device and fresh-root rows so it is not
  vacuous. Audit 12 had to measure this by hand because nothing asserted it.
* **SOURCE-SLICE:** `ledger.rs` sliced from the top of the file to its single
  `#[cfg(test)]` sentinel (line 583 of 918), asserting the forbidden API appears
  **zero times** in the sliced CODE, **with its literal name written in the
  assertion** — which lives in the TEST file and therefore OUTSIDE the slice.
  Four positive controls prove the slice is the right region and the sentinel
  count is asserted to be exactly one. **That is what dissolves the inversion a
  whole-file grep creates**, and it is why `ledger.rs:354-361`'s stated reason is
  backwards: writing the literal would turn a zero-occurrence gate RED, not make
  it pass vacuously. **`19-33`'s to correct.**

**`T-19-126`(ii):** entry 7 re-measured under real GNU tar and it reproduces —
*"Cowardly refusing to create an empty archive"*, because the re-spelling moved
`-C` last and dropped the `.` member. **The repair is an ADDED reaching entry —
`tar --create --file /tmp/t -C<ENV>/alpha .`, verified to archive
`./pr-ledger.ndjson` — and an APPENDED WR-02 note. Entry 7 STAYS; nothing is
deleted or reworded**, because a case that stops being drawn is a case that stops
being able to fail.

---

## The `SEPARATORS` re-derivation, and the three stale citations

```text
git log -L 2422,2422:src/envelope/policy.rs  ->  84a9b05  (plan 19-05), EXACTLY ONE
git log -L 2297,2297:src/envelope/policy.rs  ->  af72137  (plan 19-15), which says
                                                 NOTHING about `SEPARATORS`
```

**The CITATION is stale, not the MECHANISM moved** — round 12 added prose above
the constant. It is byte-identical; `is_separator(">")`, `("<")` and `("=")` are
all `false`; `\n` IS a separator, which is the mechanism behind this round's
correction to audit 12's heredoc row. `tests/envelope_word_set.rs` cites
`policy.rs:2422` throughout.

**The THREE stale citations under `tests/` are `19-33`'s to correct:**
`envelope_wrapper_class.rs:8357`, `envelope_carrier_reach.rs:2889`,
`envelope_interior_path.rs:2458`.

---

## The `cred.rs` alias-body route — measured, and explicitly NOT folded

```text
                                                          real git       guard
CONTROL  no `-c` at all                                   exit 128, ABSENT   —
-c credential.helper=store (T-19-121, the 2nd control)    secret PRESENT     exit 2
-c alias.q='!git -c credential.helper=store … fill' q     secret PRESENT     exit 0
```

Driven under the envelope's full posture with `19-29`'s injected
empty-`credential.helper` pair present and the **CONTROL DRIVEN FIRST**. **The
secret is recorded PRESENT/ABSENT and never transcribed** (SAFE-04).

`cred.rs:416-457`'s WHAT IT DOES NOT COVER list has **TWO** items, quoted
verbatim in the record: *"an agent that unsets `GIT_CONFIG_COUNT`, which is
D-09's stated ceiling"* and *"a later `-c credential.helper=<something>` on the
same command line, which OVERRIDES the reset and brings the secret back"*. **The
alias-body route is ABSENT from the list.**

**This is `T-19-86`'s route. It is RECORDED, never folded** into `T-19-86`'s
declared harm and never used to re-rate anything. Audit 12 declined to fold it
and so does this plan; `19-27`'s five-site attribution correction stands. The
missing list item is `19-33`'s to add.

---

## The FENCED-FILE ENUMERATION, re-derived mechanically

```text
grep -rnE '<<<|<<-|<<EOF|>&[0-9]|<&[0-9]' tests/ src/     55 hits
    every `<<<` / `<<` / `<<-` row carries a target (`x`, `EOF`, `here`, `2`,
    `0`, `1`) naming NO protected path -> NONE of them moves.
grep -rnE '<ENV>([ "]|$)' tests/                           1 hit
    `envelope_wrapper_class.rs:7834`, the PLACEHOLDER CONSTANT's own definition.
    **NO corpus entry names the envelope root alone.**
grep -rn 'PARENT of the envelope directory' tests/ src/    2 hits
    `policy.rs:10024` and `envelope_interior_path.rs:2341` — EXACTLY the two
    ancestor pins, both `19-33`'s named cross-fence exception.
grep -rn 'policy.rs:2297' tests/                           3 hits
    the three stale citations, `19-33`'s to correct.
```

**The plan's planning-time answer was CONFIRMED on all four greps.** Every
`policy.rs` unit pin was additionally checked individually against the simulated
post-fix design: the `..`-collapse rows, `./alpha/pr-ledger.ndjson`, the `alphax`
and `alpha2` siblings, the basename near-miss, the one-character-changed root,
the binary's parent and the binary's sibling **all stay `false`**; the sibling
under the root, the second alias, `/tmp` and `/` **all stay `false`**; and the
ancestor row is the ONLY one that moves.

---

## The two ORDERING pins, at deliberately DIFFERENT identifiers

**All orderings were MEASURED before any was written**, and both pins are
three-way because a two-way pin would pass if the classifier simply stopped
firing:

```text
carrier segment FIRST, force push SECOND   measured force_push_blocked  -> ASSERTED envelope_assertion_failed  (RED)
force push FIRST, carrier segment SECOND   measured force_push_blocked  -> ASSERTED unchanged                  (GREEN)
an OUTSIDE-the-root word, either order     measured force_push_blocked  -> ASSERTED unchanged                  (GREEN)
```

Pin A is the here-string carrier and pin B the ancestor, **deliberately on
different axes**, so the two together prove BOTH widenings are raised in the ONE
per-segment walk rather than in a second pass. **Round 3's principle is
DISCHARGED rather than weakened.**

---

## The axis — twelve classes to FOURTEEN, with SIX predicates extended

`MIN_CONTROL_CARRIER_CLASSES` 12 → **14**; `CONTROL_CARRIER_SLOTS` 13 → **15**;
`CONTROL_CARRIER_CASES` 84 → **107**. Both new alphabets are **FAIL-CLOSED**
(four → six). `MIN_CONFIG_RESOLUTION_CLASSES` stays at **6** and the four earlier
axes, their predicates, their degenerate-proofing and their floors are
BYTE-IDENTICAL.

**Both new classes are on the CONTROL-CARRIER axis rather than a sixth one**:
rule (a) decides on a word's PATH, so which words reach the rule and which paths
it protects are the same axis's own alphabet, not a new stage.

**SIX predicates were extended, and every one prevents a MISATTRIBUTION** —
filing a live, unruled bypass under a class the record says is CLOSED is worse
than a wrong arm, because a later reader checking whether the class still fails
would find it green and conclude the spelling was covered:

| Class | Extension | Would otherwise have been filed under |
|---|---|---|
| 1 | excludes a word naming the root EXACTLY | round 10's closed class |
| 2 | requires a PATHNAME operator | round 11's closed class |
| 5 | excludes a non-pathname redirection target | the relative class |
| 7 | the COMPLEMENT, excludes 13 and 14 | the INVARIANCE arm |
| 11 | excludes a command class 13 draws | round 11's closed class |
| 12 | excludes an interior path that IS the root, and an attached non-pathname target | round 12's closed class |

This is `19-18`'s `{v}>` blocker, `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` split,
`19-22`'s indirection/confined split, `19-24`'s re-parsed/confined split,
`19-28`'s class-2 collision and `19-30`'s complement extension — **a SEVENTH
time, and the SECOND through class 2 specifically.**

### The two class collisions the plan predicted, and TWO it did not

**Both predicted collisions behaved exactly as predicted.** A here-string entry
satisfied **class 2** (`draws_a_redirection_target_carrier`, whose predicate asks
only `is_redirection_target && starts_with(ROOT)` and does not distinguish a
pathname operator from a non-pathname one), and an ancestor entry satisfied
**class 1** (`draws_an_envelope_root_operand`, since a word that IS the root
starts with itself). Both were EXTENDED, and **neither extension vacated a cell**
— re-derived mechanically and recorded as a CHECK rather than a conclusion:
every `CONTROL_CARRIER_REDIRECTION_TARGETS` and
`..._PRESERVING` entry uses `>` or `>>`, both pathname operators; and
`grep -rnE '<ENV>([ "]|$)' tests/` returns exactly one hit, the placeholder
constant's own definition.

**TWO FURTHER COLLISIONS WERE NOT PREDICTED AND ARE REPORTED AS FINDINGS.** The
class-count fence went RED at **15 against a derived 13**, which is exactly what
an exact-equality count fence exists to do:

* **class 11** — `xargs cp /bin/true <<< <BIN>` puts the binary's own path in a
  here-string target, so the EQUALITY drew it, filing a live bypass under
  **round 11's closed class**;
* **class 12 and class 5** — the ATTACHED `>&` spelling is ONE whitespace word
  whose `/` sits at index 2, so `is_redirection_target` (which reads the
  PREVIOUS word) is FALSE for it. Class 12 drew it as an interior carrier path —
  **round 12's closed class** — and once that was fixed, class 5 drew it as a
  relative carrier, because its BASENAME is a carrier filename and it does not
  begin with `/`.

All three were extended with their reasons stated, each with its
no-entry-lost check re-derived.

### The arithmetic, re-derived with each NAIVE value beside it

**The FALL is the arithmetic proof the extension took effect:**

```text
class  1  envelope-root operand      11 -> 14   naive 20   (fall of 6)
class  2  redirection target          4 ->  4   naive  7   (fall of 3)
class  5  relative carrier            2 ->  2   naive  4   (fall of 2)
class  7  ordinary complement        24 -> 29   naive 43   (fall of 14)
class 11  the guard's own binary      5 ->  5   naive  6   (fall of 1)
class 12  interior carrier path      12 -> 13   naive 16   (fall of 3)
class 13  non-pathname target         —  ->  7
class 14  ancestor                    —  ->  7
CASES 84 -> 107   SLOTS 13 -> 15   ORDINARY 24 -> 32   INTERIOR 12 -> 13
```

**ONE DELIBERATE OVERLAP IS RECORDED rather than smoothed away.** Class 1's count
rises by THREE because the ANCESTOR-STOP controls `ls <ENV>/unrelated-sibling`,
`rm -f <ENV>/unrelated-sibling` and `rm -rf <ENV>/beta` really are absolute
literal operands under the ROOT. **That is exactly the distinction `T-19-123` is
about**: class 1's predicate is over the ROOT while rule (a)'s prefix is over
`<root>/<alias>`, and the three rows sit in the gap between them. They are
PERMITTED before and after, sit in the INVARIANCE arm, and no fail-closed
property touches them.

### The four fences, with the two new ones named

* **WORD-SET fence** (new) — counts non-pathname redirection targets carrying a
  carrier path **and** requires at least one under an operator OTHER than `<<<`,
  naming the here-string-only trap explicitly.
* **ANCESTOR-STOP fence** (new) — requires an ancestor entry in the fail-closed
  arm **and** a word at or above the root's own parent in the invariance arm, so
  a clause that walked the chain to `/` turns the file red. It also asserts the
  pair `rm -rf <ENV>` / `ls <ENV>/unrelated-sibling` **by name**.
* **NO-PROGRAM-NAMES fence** — extended to both new classes; `xargs`, `mv`,
  `find`, `chmod` and `tar` measured at zero quoted occurrences in `policy.rs`
  and `hooks.rs`.
* **PATH-PREFIX-NOT-BASENAME fence** — extended with the statement that the
  ancestor comparison is **the same component vectors the other way round**,
  bounded at the root, which is the mechanical reason it adds exactly one path;
  with five negative and three positive controls.

`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` stays
GREEN.

---

## FINDINGS AND THINGS FOUND-AND-NOT-FIXED

1. **`19-32`'s own gate is self-contradictory, and it is reported rather than
   smoothed.** It requires BOTH zero deletions under `tests/` AND that
   `MIN_CONTROL_CARRIER_CLASSES` rise 12 → 14, `CONTROL_CARRIER_SLOTS` become 15,
   `CONTROL_CARRIER_CASES` be recomputed and the per-class counts be re-derived.
   **A raised floor cannot be written without changing the line that carries it**,
   and git counts a changed line as one deletion. **The eight changed lines are
   ALL RAISES** — 84→107, 13→15, 12→14, 12→13, 24→32, 11→14, 24→29, 12→13 — and
   **not one alphabet entry, property or assertion was removed, reworded away or
   narrowed.** The intent `19-32` states in its own words is *"nothing may be
   removed, reworded away or narrowed … a case that stops being drawn is a case
   that stops being able to fail"*, and **a floor that RISES is the opposite of
   narrowing.** A new pin,
   `no_control_carrier_floor_or_class_count_fell_when_round_13_re_derived_them`,
   asserts that mechanically over seventeen floors and nine class counts.
   `tests/envelope_word_set.rs` shows **+3339 / −0**.
2. **Two class collisions the plan did not predict** (class 11 and the
   class-12/class-5 pair), found by the count fence going red at 15 against a
   derived 13 — reported above with the entries that reached them.
3. **The heredoc BODY form is already refused** — a correction to audit 12,
   written beside its row.
4. **`>&` with a path REACHES** and was not predicted to — bash's `&>word`
   fallback. It became the non-`<<<` operator the alphabet needed.
5. **Three of the five non-pathname operators reach NOTHING** — `<<` and `<<-`
   with the path as the delimiter, and `<&`. **RECORDED rather than asserted**, on
   the discipline that a spelling the guard permits but the shell does not reach
   is not a bypass.
6. **Audit 12's 5 s crossing point does not reproduce exactly** — 16 KB answers
   in 3 904 ms here and the deadline is crossed between 18 KB and 20 KB. The
   mechanism is identical; the number is a fact about a machine.
7. **Two fixture bugs of this plan's own making**, caught by the naming contract
   and reported above: a twin pair driven against the wrong envelope root, and a
   credential fixture that measured the control twice.
8. **A documented `driver_reattach` flake fired on one full-suite run and NOT on
   another** — verbatim output above, both observations recorded, not fixed,
   relation to this round claimed in neither direction, and **absence is not
   evidence it is fixed.**
9. **`rm -rf <binary-parent>`** — a `T-19-116` route audit 12 added, exit 0,
   RECORDED in neither direction. `word_is_exactly`'s reasoning argues only that
   a PREFIX would be wrong and never that the parent's own DELETION removes the
   binary — **an HONESTY gap `19-33` repairs in that doc, not a rule gap.**

**No row came up GREEN where this plan expected RED.** All ten
`after_19_33_*` rows and the one `_after_19_33` axis row were measured at exit 0
(or at the wrong identifier) before they were written and are RED now.

---

## `SECTION_ENVELOPE` — RE-MEASURED and NOT edited

```text
whitespace tokens : 211  of an UNRAISED cap of 215
widest line       :  74  of a cap of 80
first `Guaranteed` clause : still TRUE, still opening "As started"
```

**"As started" states what the envelope ESTABLISHES and stops**, so neither
`T-19-122` nor `T-19-123` falsifies it — and the `Not guaranteed` half already
generalises to *"the files and the binary this envelope runs on"*, which covers
BOTH of this round's routes. **No new disclosure is owed and none is written.
`src/envelope/advisory.rs` shows zero diff lines and `19-33` is prohibited from
opening it too.** Headroom that is spent cannot be got back.

---

## Carried forward untouched

`AR-19-04`, `AR-19-05` and `T-19-23` are untouched — not un-accepted, not
re-rated, not renumbered, not marked closed. **`T-19-17r` is OUTSTANDING for the
SEVENTEENTH time**: `grep -cE '^\| AR-19-13 \|'` over `19-SECURITY.md` is **0**,
no Accepted-Risks-Log row was added, and **the word "accepted" is applied to it
nowhere** — not in a test name, a comment, this SUMMARY or the appended
subsection. **This plan did NOT accept it.**

`C-08`'s, `T-19-120`'s and `T-19-124`'s behavioural halves stay UNMEASURED.
`C-11` … `C-15` keep control (e) with **no `pr_cap_*` clamp**. `T-19-110`,
`T-19-115` and the `glab --host` cell are unchanged, `FORGE_VALUE_OPTS` keeps
`--hostname`, and **`glab` is confirmed NOT INSTALLED** so no pin that would skip
was written. **No crate was added** — `Cargo.toml` and `Cargo.lock` appear in no
commit of this plan.

---

## What remains uncovered

**`T-19-86` FIRST — open at `high` by explicit user scoping decision**, entirely
unremediated; its rows are at exit 0 and its pins are green and UNMODIFIED, and
the `!`-bodied alias credential route measured here is its route, recorded and
not folded.

Then, open at `high`: **`T-19-91`**, **`T-19-111`**, **`T-19-112`**,
**`T-19-116`** (which gained TWO routes this audit and whose second,
`rm -rf <binary-parent>`, closing `T-19-122` does NOT close), **`T-19-121`**,
**`T-19-122`** and **`T-19-123`**.

Then **`T-19-113`**, **`T-19-115`**, **`T-19-124`** (`medium`), **`T-19-125`**
and **`T-19-126`** (`low`), **`T-19-96`**, **`T-19-110`**, **`T-19-74`**,
**`T-19-84`**, **`T-19-85`** and **`T-19-61` … `T-19-73`**.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

---

## Self-Check: PASSED

```text
tests/envelope_word_set.rs                                   FOUND
tests/envelope_wrapper_class.rs                              FOUND
.planning/.../19-SECURITY.md (plan-19-32 record appended)    FOUND
.planning/.../deferred-items.md (plan-19-32 section)         FOUND
commit 172b1d3                                               FOUND
commit 604c644                                               FOUND
```
