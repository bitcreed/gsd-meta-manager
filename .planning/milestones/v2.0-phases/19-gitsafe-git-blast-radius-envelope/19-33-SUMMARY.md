---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 33
subsystem: envelope
tags: [security, guard, policy, word-set, path-set, ancestor, linearity, honesty]
status: complete

requires:
  - "19-32's committed RED corpus and its handoff contract"
provides:
  - "a THIRD word class: LITERAL NON-PATHNAME redirection targets, on a SECOND `Segment` field"
  - "an ANCESTOR clause beside the PREFIX one, bounded at the envelope ROOT"
  - "a candidate scan whose work is LINEAR in the word's length, with a fail-closed `CANDIDATE_SCAN_WORK_CEILING` a test DRIVES"
  - "four corrected claims under the WR-02 discipline"
  - "the residue restated as a CONDITION over THREE axes, with no count on any of them"
affects:
  - src/envelope/policy.rs
  - src/envelope/ledger.rs
  - src/envelope/cred.rs
  - tests/envelope_word_set.rs
  - tests/envelope_interior_path.rs
  - tests/envelope_carrier_reach.rs
  - tests/envelope_wrapper_class.rs

tech-stack:
  added: []
  patterns:
    - "a second `Segment` field rather than a widened grammar flag, decided by the fenced-file enumeration"
    - "an ancestor clause as the same component vectors compared the other way round"
    - "suffix folds with structure sharing, to make a per-candidate normalisation linear"
    - "a fail-closed work ceiling as a BACKSTOP beside a linearity BOUND"

key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/envelope/ledger.rs
    - src/envelope/cred.rs
    - tests/envelope_word_set.rs
    - tests/envelope_interior_path.rs
    - tests/envelope_carrier_reach.rs
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md

decisions:
  - "T-19-122 implemented as a SECOND `Segment` field and a THIRD word class, never as a widened `pathname_target` — the choice was decided by which fenced pins each design turns RED, not by taste"
  - "T-19-123 implemented as an ANCESTOR clause beside the PREFIX one, bounded at the envelope ROOT on ownership grounds, never as the prefix widened to the root"
  - "T-19-124 bounded on LINEARITY of the scan's work, with a fail-closed ceiling as the backstop; the task was NOT severed"
  - "the ledger.rs forbidden-API name is still not spelled in that comment — but for a checkable reason about where the pin's slice begins, not the vacuity claim it used to give"
  - "audit 12's finish-line sentence is NOT written here; that is audit 13's judgement"

metrics:
  duration: "one session"
  completed: "2026-09-04"

actuals:
  tokens: 96000
  tasks: 5
  commits: 5
---

# Phase 19 Plan 33: the word the guard deletes, the directory destroyed by naming its parent, and the scan that bounded the wrong quantity — Summary

A third word class and an ancestor clause at the one existing decision point, a
candidate scan made linear with a fail-closed work ceiling a test drives, four
corrected claims, two authorised cross-fence pin moves, and the residue restated
over all three axes — gated green at 1924/0 with the arithmetic checked.

## FIRST, and in these words

**`/gsd-secure-phase 19` is NOT cleared by this plan.**

`T-19-86` remains **OPEN at `high`** by explicit user scoping decision, entirely
unremediated. `T-19-91` remains **OPEN at `high`**. `T-19-111` remains **OPEN at
`high` with NO rule**. `T-19-112` is narrowed further and **not closed**.
`T-19-116` is **NARROWED and NOT closed**. `T-19-121` remains a bounded residue
of a control that landed.

**Nothing in this plan is described as a closure.** Whether `T-19-122` …
`T-19-126` close is **audit 13's judgement rather than this plan's claim**; this
plan says NARROWED-or-CORRECTED and states what remains open for each.

**And this plan does NOT write audit 12's finish-line sentence.** Audit 12 wrote:
*"Not yet. Five of the eight are. But `T-19-122` and `T-19-123` are neither —
unmodelled classes with driven harm. When those two are ruled or accepted, that
sentence becomes true and will be worth writing."* **Both are ruled here.** The
sentence is still not written, because whether the open set has become the
human-decision items plus the deliberately-unruled residues is audit 13's to say,
and a plan that certified its own round on that question would be this phase's
certification defect in its purest form.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" appears anywhere in this plan's output.

---

## `19-32`'s complete RED list, confirmed STILL RED before any production line moved

The full suite was run against the unmodified tree at `07c551a` with
`--no-fail-fast` **before the first edit**. Verbatim:

```text
passed=1903 failed=11 ignored=13 sum=1914
48 result lines, 19 envelope_* binaries

a_command_naming_a_carrier_in_a_word_class_or_a_path_set_rule_a_has_no_clause_for_is_refused_after_19_33
after_19_33_a_here_string_word_carrying_the_path_at_a_non_zero_index_is_refused
after_19_33_a_here_string_word_naming_the_envelope_directory_is_refused
after_19_33_a_here_string_word_naming_the_guards_own_binary_is_refused
after_19_33_a_non_pathname_operator_other_than_a_here_string_reaches_the_same_way
after_19_33_a_word_naming_the_ancestor_of_the_envelope_directory_is_refused
after_19_33_ordering_pin_a_the_here_string_carrier_wins_its_own_segment
after_19_33_ordering_pin_b_the_ancestor_wins_its_own_segment
after_19_33_the_ancestor_spellings_that_are_not_rm_are_refused_the_same_way
after_19_33_the_candidate_scan_is_linear_in_the_words_length
after_19_33_the_two_segment_composite_naming_the_ancestor_is_refused
```

**ELEVEN rows, every one on `19-32`'s recorded list, and NOT ONE already green.**
Every failure carried its `after_19_33` marker. The per-binary counts matched
`19-32`'s record exactly — `envelope_word_set` 28/10, `envelope_wrapper_class`
59/1, `envelope_config_resolution` 30/0, `envelope_interior_path` 41/0 — and
`passed + failed` was **1914**, `19-32`'s run-B number. No documented flake fired
on that run.

### The after-verdict for every row

| row | before | after |
|---|---|---|
| three here-string envelope-directory rows | RED | GREEN after Task 1 |
| the here-string BINARY row (`C-10`, layers 2 and 3) | RED | GREEN after Task 1 |
| the `>&` rows (a SECOND non-pathname operator) | RED | GREEN after Task 1 |
| the non-zero-index here-string row | RED | GREEN after Task 1 |
| ordering pin A — the here-string carrier | RED | GREEN after Task 1 |
| the ancestor row and its trailing-slash spelling | RED | GREEN after Task 2 |
| the four non-`rm` ancestor spellings (`mv`, `find`, `chmod`, `tar -C`) | RED | GREEN after Task 2 |
| the two-segment ancestor composite | RED | GREEN after Task 2 |
| ordering pin B — the ancestor | RED | GREEN after Task 2 |
| the axis row in `envelope_wrapper_class.rs` | RED | GREEN after Task 2 |
| the linearity ratio (SEVERANCE-RETIRABLE) | RED | GREEN after Task 3 |

**The transition, per file:** `envelope_word_set` 28/10 → 33/5 → 37/1 → 42/0;
`envelope_wrapper_class` 59/1 → 60/0.

## `19-32`'s PERMITTED list, re-checked after EVERY task — the zero-move diff

The PERMITTED set is carried by the green half of the same fenced files, so the
check is mechanical rather than narrative. **`envelope_word_set`'s PASSING count
never fell** (28 → 33 → 37 → 42) and **every other fenced binary held its exact
count at every step**:

```text
                          base    T1     T2     T3     T4/gate
envelope_word_set        28/10   33/5   37/1   42/0   42/0
envelope_wrapper_class    59/1   59/1   60/0   60/0   60/0
envelope_interior_path    41/0   41/0   41/0   41/0   41/0
envelope_config_resolution 30/0  30/0   30/0   30/0   30/0
envelope_carrier_reach    39/0   39/0   39/0   39/0   39/0
envelope_control_carrier  37/0   37/0   37/0   37/0   37/0
envelope_literal_decision 43/0   43/0   43/0   43/0   43/0
envelope_argv_deletion    20/0   20/0   20/0   20/0   20/0
  (and the ten others, all unchanged)
```

Those carry `cat <<<x`, `cat <<< /tmp/x`, `git <<<x push --force origin main` at
`force_push_blocked`, `ls <ENV>/unrelated-sibling`, `rm -f <ENV>/unrelated-sibling`,
`ls <ENV>/beta`, `rm -rf <ENV>/beta`, the user-set-root case, `ls <ENV-PARENT>`,
`ls /tmp`, `ls /`, `df /`, `ls /home`, both binary EXACT-PATH controls,
`xargs rm -rf <<< of=/tmp/g/x`, `echo x >&/tmp/plain-outside`,
`git --git-dir=/tmp/g status`, `dd of=/tmp/g/./x`, the 100 000-character
no-slash word, the symlinked-to-regular ledger, the fresh-root ledger and both
`T-19-86` `!`-bodied rows.

**ZERO rows moved from permitted to refused at any step.**

---

## The commits, in order — and the doc task is ORDERED LAST

```text
cb02cdf  feat(19-33): the THIRD word class
           src/envelope/policy.rs                       +392 / -25
9748141  feat(19-33): the ANCESTOR clause and both authorised pin moves
           src/envelope/policy.rs                       +353 hunks
           tests/envelope_interior_path.rs               +89 / -3 comments+1 row
47c0f1b  perf(19-33): the candidate scan bounded on WORK
           src/envelope/policy.rs
           tests/envelope_word_set.rs                   +175 / -0
5e65ad1  docs(19-33): the four honesty repairs, the sweep, the THREE-AXIS condition
           src/envelope/policy.rs   +481
           src/envelope/ledger.rs    +45 / -                (the KIND comment only)
           src/envelope/cred.rs      +32                    (the list only)
           tests/envelope_carrier_reach.rs   6 lines, comment-only
           tests/envelope_interior_path.rs   6 lines, comment-only
           tests/envelope_wrapper_class.rs   5 lines, comment-only
0f66086  docs(19-33): the execution record, the registrations, and the gate
           19-SECURITY.md   +621   deferred-items.md   +152
```

**The doc task is ORDERED AFTER all three rule tasks**, on the ordering
discipline the `19-30`/`19-31` plan-check established. Task 4 read the tree
first — `git log --oneline`, and `grep -cE 'const CANDIDATE_SCAN_WORK_CEILING'`
returning **1** — so **every repaired claim and every axis of the condition
records an OBSERVED outcome rather than a prediction.**

---

## `T-19-122` — the design as implemented, with the derivation that decided it

A SECOND `Segment` field, `non_pathname_redirection_targets`, carrying every
LITERAL word the simple command names after a NON-PATHNAME redirection operator;
populated in the SAME `consume_redirection` arm from the SAME
`skip_redirection_target` by-product — one `else if` — and chained as a THIRD
`.chain()` at the ONE existing reading site.

| Design | What it does | What it turns RED |
|---|---|---|
| **(A)** widen `RedirectionOperator::pathname_target` | one flag flip | `only_a_literal_pathname_target_reaches_the_segment_and_the_tokens_do_not_move` and `the_operator_grammar_says_which_targets_are_pathnames_and_which_are_not` |
| **(B)** a SECOND field + a THIRD word class at the SAME site | one field, one `else if`, one `.chain()` | **nothing** |

**Design (B) was implemented, and BOTH fenced pins were run IMMEDIATELY after
the change and are GREEN**, including the SEGMENT-COUNT rows. **Neither was
edited.**

**The rejected third design, costed rather than dismissed.** *Treat every
deleted-and-unrecorded word as UNRESOLVABLE* would deny `cat <<< hello`,
`sort <<< "$x"` and every fd duplication on the line — an outage rather than a
boundary (AR-19-11). `cat <<<x` and `cat <<< /tmp/x` stayed permitted.

**The principled restatement, now in the field's own doc:** the predicate's
subject is what a line **NAMES**, not what the shell does with a word. The
pathname/non-pathname distinction is a fact about the SHELL's USE of the word;
`protected_carrier_named` decides on the word's TEXT. **Bash's split stays
byte-identical and is read from BOTH sides for different reasons** — the pathname
side because the shell will `open(2)` the word, this side because a data channel
feeding `xargs` still names a path — and the doc says so in place.

**The disclosed over-refusal, with its two permitted controls.**
`cat <<< <ledger>` becomes refused — a here-string naming a protected path is
refused even where the program would only ever have read it as data. Same family
as `cat <ledger>`, fails CLOSED, bounded to this run's own paths. **`cat <<<x`
and `cat <<< /tmp/x` stay permitted, which is what keeps this a PATH class rather
than a here-string ban.**

**ZERO behavioural change, read out of the diff:** `redirection_operator`'s
twelve-operator match, `RedirectionOperator::pathname_target`,
`skip_redirection_target`'s parsing, `Segment::redirection_targets`,
`segment.tokens`, `Token`, `SEPARATORS` and `is_separator` are untouched.
`git x2>/tmp/o push --force origin main` is still exit 0 as **exactly ONE
segment**; `git <<<x push --force origin main` is still exit 2
`force_push_blocked`.

---

## `T-19-123` — the design as implemented, with the path set, the stop and the cost

```text
a candidate is refused when it is
  (1) UNDER `<root>/<alias>`, component-wise             <- the existing PREFIX, unchanged
  (2) a proper ANCESTOR of `<root>/<alias>` that is
      itself AT OR UNDER `<root>`                        <- the NEW clause
  (3) EQUAL to `current_exe()`                           <- the existing EQUALITY, unchanged
```

**It grew the protected set by EXACTLY ONE path — `<root>`.** The alias is a
plain single path component and `ledger_path_in` refuses anything else, so a
candidate that is both a proper prefix of the directory and at or under the root
can only be the root itself. **It is written in its general form anyway**, so it
stays correct if the directory ever becomes deeper.

**Where it stops: `<root>`. Why: OWNERSHIP.** This tool creates the root and it
holds only the alias directories it created. Above it the chain is
`~/.local/share`, `$HOME`, `/tmp`, `/` — shared with everything the user has. A
clause reaching them would refuse `ls /`, `df /`, `du -sh $HOME` and `ls /tmp`:
**not a boundary but an outage** (AR-19-11), the same argument `word_is_exactly`
gives one path over. **The root is taken as an EXPLICIT input**, derived once in
`word_is_within` as the given directory minus its last component, so the code
cannot silently drift into a prefix over the root.

### The four-row ANCESTOR-vs-PREFIX discriminator, all rows re-driven

```text
                                    ancestor (mandated)   prefix widened to <root> (forbidden)
rm -rf <root>                       REFUSED  <- now        REFUSED
rm -rf <root>/alpha                 REFUSED                REFUSED
ls <root>/unrelated-sibling         PERMITTED <- held      REFUSED
ls <root>/beta  (a second alias)    PERMITTED <- held      REFUSED
user-set GSD_MM_ENVELOPE_ROOT over
a shared directory; ls <that>/x     PERMITTED <- held      REFUSED
ls <root-parent> / ls /tmp / ls /   PERMITTED <- held      PERMITTED
```

**The user-set-root row is reachable CONFIGURATION rather than a thought
experiment**, and it stayed permitted.

### The residue ABOVE the root

An ancestor above `<root>` — `rm -rf /tmp` when the root is `/tmp/xyz` — reaches
the same nine carriers and **NO RULE IS WRITTEN FOR IT.** **Registered,
disclosed, UNACCEPTED**, named on axis 3 of the restated condition, and handed to
no pin, schedule or version witness.

**NO ANCESTOR CLAUSE IS WRITTEN FOR THE BINARY.** `word_is_exactly` stays an
EQUALITY and was not changed by the rule task.

---

## Both authorised pin moves, with each reasoning quoted BEFORE and AFTER

Audit 12's own words: *"the code does exactly what the pin says. What is missing
is the threat row and the residue clause, not the implementation … the unit pin
must MOVE either way, and moving it is a design change, not a test edit."*

**BEFORE — `policy.rs`, NEGATIVE list:**

> `("/tmp/envroot", "the PARENT of the envelope directory is not under it. The`
> `boundary is the directory this run owns, not everything beside it")`

**BEFORE — `tests/envelope_interior_path.rs`, NEGATIVE list:**

> `("/tmp/envroot", "the PARENT of the envelope directory is not under it")`

**AFTER — both rows sit in the POSITIVE list, with their reasoning REWRITTEN in
place under WR-02 and NOT DELETED**, in three parts:

* **Why it was right when written** — the boundary round 11 declared was the
  directory this run OWNS, and a PREFIX widened to the parent really would have
  been wrong: it would refuse every sibling under the root, a concurrent run's
  second alias directory and, `GSD_MM_ENVELOPE_ROOT` being user-settable, a whole
  subtree of a user's own files. *"Not everything beside it"* is still true and
  still enforced — the `alphax` and `alpha2` rows did not move.
* **What changed** — the harm the permit admitted was MEASURED: `rm -rf <root>`
  takes the same nine carriers one component up, driven twice with a control
  beside every leg, moving a bare remote's `main` (`c13f9ea` → `371a2f5`,
  `0658c51` → `140421f`) and resetting a FIRED pull-request cap to PERMITTED.
* **What is true now** — the boundary is the directory this run owns **plus its
  ancestors up to the root this run was given, and no further**, with the reason
  the chain stops there.

**A STOP ROW WAS ADDED BESIDE EACH**, so the move is bounded by an assertion
rather than by prose: `policy.rs` gains a one-character-different-root row in the
negative list plus a whole
`the_ancestor_clause_stops_at_the_root_and_the_stop_is_pinned` fn pinning `/tmp`,
`/` and a different root's alias directory `false`;
`tests/envelope_interior_path.rs` gains `/tmp` — the root's own parent — and
`/tmp/envrooz` to its negative list.

**THE BINARY'S-PARENT ROW DID NOT MOVE**, and its comment gained one sentence
saying why: no ancestor clause is written for the binary, because its directory
is shared. **NO THIRD ROW UNDER `tests/` MOVED THAT IS NOT A COMMENT** — checked
mechanically: **six** deleted lines under `tests/` across the whole plan, **five
comments and one the authorised ancestor row.**

**One thing the move required that the plan did not name.**
`every_fenced_unit_pin_…` runs against a test-local `simulated_protected`, so the
row could not move until that simulation carried the ancestor arm too. That is an
ADDITION inside the function (its signature is byte-identical, and the only
deleted lines are its two `///` doc lines), and it moves **exactly one row** —
verified by hand against all thirty-one other words that function is driven over
before the edit was made.

---

## `T-19-124` — the bound LANDED. It is on WORK, and the ceiling is DRIVEN

**Task 3 was NOT severed.** `grep -cE 'const CANDIDATE_SCAN_WORK_CEILING'
src/envelope/policy.rs` is **1**, read from the TREE — not from a marker file and
not from a commit message. **The gate's zero-failure branch was the one taken.**

**The shape.** `CandidateScan` normalises every `/`-anchored candidate in ONE
RIGHT-TO-LEFT pass with STRUCTURE SHARING. Splitting once on `/` makes the
candidates the SUFFIX FOLDS of one part list, and a suffix fold obeys a three-arm
recurrence: a `""`/`"."` part leaves the answer unchanged; a `".."` leaves it
unchanged and owes one more pop to whatever precedes; a name survives if nothing
is owed and is consumed by the owed pop otherwise. One reverse pass records, per
part, whether it survives and which surviving part follows it — so every
candidate's component list is a suffix of one shared chain, and every comparison
walks at most as many links as a PROTECTED path has components.

**The candidate SET is unchanged and is ITERATED from `slash_anchored_candidates`
rather than re-derived**, so the two cannot disagree; only the number of times
the same characters are walked moved.

**NO VERDICT MOVED, mechanically and behaviourally.**
`containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte` and
`every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule`
are both **GREEN**; `19-32`'s whole cost half and rounds 4–12's pin family were
re-driven unchanged; `envelope_config_resolution` is 30/0.

### The post-fix curve, on THREE trees

The `dd17bfb` control binary was **REBUILT** by `git archive dd17bfb | tar -x`
into a directory outside the repository and built there — **the working tree was
never checked out, reset or stashed.**

```text
slashes    bytes      19-32 HEAD        post-fix      dd17bfb (rebuilt)
  1 000    2 000           68 ms          10 ms           5 ms
  2 000    4 000          249 ms          10 ms           5 ms
  5 000   10 000        1 524 ms          11 ms           7 ms
  8 000   16 000        3 904 ms          13 ms           8 ms
 20 000   40 000       24 425 ms          27 ms          14 ms
 50 000  100 000      151 249 ms          48 ms          28 ms
100 000  200 000              —           99 ms           —
400 000  800 000  NO ANSWER in 300 s     184 ms           —
100 000 chars, NO slashes   29 ms         27 ms          21 ms

ratio t(8 000)/t(2 000)      15.7          1.22 (threshold 8)      1.22
```

**The post-fix tree is on the CONTROL's curve rather than on round 12's.** The
threshold of 8 sits halfway between the ≈4× a linear scan predicts and the ≈16× a
quadratic one predicts; measured **1.22** out of band and **1.43** under full
suite load, against **15.7** before. Load moves the ratio DOWN because process
startup is a fixed additive term in both measurements, so it cannot manufacture a
green.

### The fail-closed ceiling, and it CAN fire

`CANDIDATE_SCAN_WORK_CEILING` bounds the work ONE `protected_carrier_named`
invocation may spend — **carried across every word of the segment and both path
halves**, because a per-word or per-segment budget is a bound over a quantity the
request MULTIPLIES. On exhaustion the predicate has NOT established that no
protected path was named, so it fails closed at
`ParkReason::EnvelopeAssertionFailed`. **No new `ParkReason`, no
`HookBypassBlocked`, no `PrCapExceeded`.**

Driven end to end through the built binary by
`after_19_33_the_fail_closed_work_ceiling_can_fire_and_this_row_drives_it`:

```text
500 KB slash-dense (250 000 slashes)   exit 2  envelope_assertion_failed   144 ms
500 KB with NO slashes (one `/`)       exit 0                              130 ms
1.5 MB (over the input bound)          exit 2  "larger than 1048576 bytes"   8 ms
300 KB slash-dense (150 000 slashes)   exit 0                              137 ms
```

**The second row attributes the refusal to the SCAN's WORK rather than to the
input's LENGTH** — without it the row would pass against a rule that simply
refused long words. **The third keeps the ceiling and `MAX_GUARD_REQUEST_BYTES`
distinguishable**, so neither can be credited with a refusal the other produced
(D-24).

`19-32`'s two RECORDED rows were **promoted to asserted**: the 50 000-slash row
(151 249 ms → 66 ms, permitted) and the 800 KB row (no answer in 300 s → 181 ms).
The 800 KB row asserts that it ANSWERS inside `GUARD_TIMEOUT_SECS` and **RECORDS
which bound produced the verdict**, because that is a fact about the ceiling's
VALUE rather than about the reformulation.

**`MAX_GUARD_REQUEST_BYTES`, `GUARD_TIMEOUT_SECS` and `MAX_LEDGER_BYTES` KEEP
THEIR VALUES** — a bound over the wrong property is not corrected by moving a
number — and **`hooks.rs` was NOT OPENED**: no budget is threaded through
`guard`, `guard_in` or `classify_segments`.

**THE BEHAVIOURAL HALF — what the agent CLI does with a `PreToolUse` hook that
overruns its registered timeout — stays UNMEASURED and is claimed in NEITHER
direction**, on the same discipline `C-08`'s and `T-19-120`'s behavioural halves
are held to.

**A design observation, recorded rather than smoothed.** The ceiling's refusal
reuses `ProtectedPath::EnvelopeDirectory` and therefore the existing carrier
message, because the plan mandates no new `ParkReason` and exactly TWO recovery
lines. The reason identifier and the mechanism are both right; **the message
names the protected path rather than the ceiling.** The over-refusal is disclosed
explicitly in the predicate's cost section, and the fact that the message does
not distinguish the two causes is recorded here rather than left to be met.

---

## The RESTATED THREE-AXIS CONDITION, quoted verbatim from the predicate's doc

> **AXIS 1 — WHICH WORDS REACH THE READER.** This predicate reads every LITERAL
> word of the segment that is not an operator (`Segment::tokens`), every LITERAL
> PATHNAME redirection target (`Segment::redirection_targets`), and every LITERAL
> NON-PATHNAME redirection target (`Segment::non_pathname_redirection_targets`).
> **It is silent about a word a redirection production that did not COMPLETE
> produced**, which fails closed at `Token::redirection_unresolvable` rather than
> being read.
>
> **AXIS 2 — WHAT THE READER SEES IN A WORD IT RECEIVED.** It reads every
> `/`-ANCHORED SUBSTRING of that word's text, normalised. **It is silent about a
> word the SHELL MAY REWRITE**, because the guard cannot know its final text;
> **about a word whose TEXT CARRIES NO ABSOLUTE PATH ANYWHERE**, because there is
> nothing in it to normalise; **and about a word that reaches a protected path
> ONLY THROUGH A LINK**, because no lexical reading of the text names it.
>
> **AXIS 3 — WHICH PATHS THE COMPARISON PROTECTS.** A candidate UNDER this run's
> envelope directory, a candidate that is a proper ANCESTOR of it AT OR UNDER
> this run's envelope ROOT, and a candidate EQUAL to this run's own binary. **It
> is silent about an ancestor ABOVE the root — `~/.local/share`, `$HOME`, `/tmp`,
> `/` — and about the binary's PARENT and every ancestor of it**, because
> refusing a word that names one of those is an outage rather than a boundary
> (AR-19-11), which is the same reason the binary half is an EQUALITY.

**NO COUNT is written on any axis**, asserted mechanically by
`the_residue_is_a_condition_over_three_axes_and_carries_no_count_on_any_of_them`,
which slices the condition block itself and forbids every count word inside it.
The spellings are INSTANCES. **It is handed to NO pin, NO schedule and NO version
witness** — `19-25`'s witness schedules a re-audit against a FUTURE GIT and every
clause here is reachable by the driven agent TODAY. **It was written LAST**, in a
task ordered after all three rule tasks, so it records an observed outcome.

A WR-02 note records what the one-axis condition said, **why it was right** (round
12's defect WAS the reading, and every clause of it is still here on axis 2) and
**what it omitted** — it named the word set and the path set as PREMISES and then
said nothing about either.

---

## All four honesty repairs, quoted BEFORE and AFTER

**Each was required regardless of whether any rule landed**, and each was written
in a task that read the tree first.

**(a) `T-19-125` — the disclosed cost.**

> BEFORE: *"Any word whose text **contains** THIS RUN'S OWN envelope directory or
> THIS RUN'S OWN binary path **as a `/`-anchored substring** is now refused …"*
>
> AFTER: *"Any word one of whose `/`-ANCHORED SUBSTRINGS **NORMALISES** to a path
> under this run's envelope directory, to an ANCESTOR of that directory at or
> under this run's envelope ROOT, or EQUAL to this run's own binary, is refused
> …"*

WR-02 reasoning: it was right-SOUNDING and one operation short — the comparison
NORMALISES, so `dd of=<env>/./<alias>/x` and `dd of=<env>/zzz/../<alias>/x` are
both refused while **neither contains `<env>/<alias>` as a substring at all**.
The round's three new cost members were added beneath it: the here-string, the
envelope ROOT, and the work ceiling.

**(b) `envelope_carrier_refusal`'s recovery lines.**

> BEFORE: *"To proceed: name a path outside that directory."*
>
> AFTER, both messages: the ancestor case is named; the path is stated to be read
> after NORMALISATION and after a redirection operator **pathname or not**; and
> the recovery splits in two — *"if the command meant to reach that path, name
> one outside it … if it did not, the path is somewhere in a word that only
> CARRIES it — in a value, a pattern, a message or a URL — and rewriting that
> word … lets the command through unchanged in every other respect."*

**No command is quoted back (SAFE-04)**, the protected path is still named, and
`git config --get core.hooksPath` and `command -v` still report the paths.
**Grepped BEFORE editing and recorded: no test asserts the wording** — the two
sites in `tests/envelope_word_set.rs` are `record_only` prints that report
PRESENT or ABSENT either way.

**(c) `word_is_exactly`'s parent-deletion residue.**

> BEFORE the doc argued only that a PREFIX over the binary's parent would be
> wrong.
>
> AFTER it adds, at the same weight: *"The reasoning above argues only that a
> PREFIX would be wrong. It never states that the PARENT'S OWN DELETION removes
> the binary — and it does."*

`rm -rf <binary-parent>` is exit 0, is a registered `T-19-116` route, and no
ancestor clause is written there for the same shared-directory reason. **The
repair explicitly says it is an HONESTY repair and NOT a mitigation.**

**(d) `T-19-126`(i) — `ledger.rs`'s inverted gate reason.**

> BEFORE: *"The API name is deliberately not spelled out: a verify step greps
> this file for that literal and asserts it appears zero times, so writing it here
> to forbid it would make the gate pass vacuously on the very mistake it exists
> to catch."*
>
> AFTER, under WR-02: **both halves were wrong** — the stated effect is INVERTED
> (writing the literal to forbid it turns a zero-occurrence gate **RED**), and the
> control it named was a **plan-time VERIFY STEP, run once**, which is not a
> standing control at all.

**The two STANDING pins `19-32` wrote are named in place**: the BEHAVIOURAL pin
driving a symlinked-to-regular ledger permitted and counted **through the link**,
and the production-SLICE pin that CAN name the API because the name lives in the
test file below the sentinel.

**(e) `cred.rs`'s WHAT IT DOES NOT COVER list gains a THIRD item.**
`git -c alias.q='!git -c credential.helper=store credential fill' q` is exit 0
and returned the ambient secret under full envelope posture (presence recorded,
**never transcribed** — SAFE-04). The by-name clause does not reach it because the
key sits inside an alias BODY: the word the guard reads is `alias.q=…`, whose
section is `alias` and whose final component is `q`.

**RECORDED AS `T-19-86`'s ROUTE AND EXPLICITLY NOT FOLDED INTO IT**, not used to
re-rate anything. `19-27`'s five-site attribution correction stands; audit 12
declined to fold it and so does this repair. The item **names the rule that does
not reach it** rather than asserting a layer governs it, which is the discipline
that bullet's own WHY THIS BULLET IS WORDED THIS WAY paragraph states.

---

## The `SEPARATORS` citation sweep — COMMENT-ONLY, three sites, mechanically gated

Both `git log -L` results were **RE-DERIVED independently** rather than inherited:

```text
git log -L 2422,2422:src/envelope/policy.rs  ->  84a9b05  (plan 19-05), EXACTLY ONE
git log -L 2297,2297:src/envelope/policy.rs  ->  af72137  (plan 19-15), which says
                                                 NOTHING about `SEPARATORS`
```

**The CONSTANT is byte-identical and the CITATION was stale** — round 12 added
prose above it. Three sites corrected, each now citing `policy.rs:2422` with the
re-derivation written beside it: `tests/envelope_wrapper_class.rs`,
`tests/envelope_carrier_reach.rs` and `tests/envelope_interior_path.rs`. **Every
changed line is a comment; not one assertion moved.**

---

## The slice placement, with a positive control naming every helper

Every helper this round added sits **AT OR AFTER** the literal
`fn lexical_absolute_components(word: &str)` anchor and therefore INSIDE
`the_carrier_predicate_asks_the_filesystem_nothing_and_its_own_source_says_so`'s
region, **and a POSITIVE CONTROL NAMES EACH** — one for `struct CandidateScan`
and its one pass, one for `fn is_ancestor_within_root` — beside the four already
there. **`lexical_absolute_components`' SIGNATURE TEXT is unchanged.** No new
helper calls `canonicalize`, `read_link`, `metadata`, `symlink_metadata`,
`exists`, `current_dir`, `current_exe` or `Command::new`; the predicate stays
PURE and normalises LEXICALLY, following no link. Every probe lives in a test.

---

## What closing `T-19-122` does and does NOT do for `T-19-116`

**`T-19-116` is NARROWED and stated OPEN at `high`, and the residues are stated
at the same weight as the narrowing.**

* **CLOSED by the third word class:** `xargs cp /bin/true <<< <BINARY>` and its
  `-I{}` spelling.
* **STILL OPEN, exit 0:** **`rm -rf <binary-parent>`** — no ancestor clause is
  written for the binary — beside the four original residues: the
  expansion-borne, tilde, relative and `PATH`-symlink spellings.

---

## Mechanism pins and byte floors, re-measured across rounds 4 through 12

All GREEN and non-dead. **The two this round sat closest to, called out by
name:**

* **Round 6's DELETION MODEL** — the third word class sits closest to it. Its
  over-deletion control `git x2>/tmp/o push --force origin main` is still exit 0
  **as exactly ONE segment**, with `git >/dev/null push --force origin main` and
  `git 2>/dev/null push --force origin main` unchanged beside it. `segment.tokens`
  is byte-for-byte what it was, `SEPARATORS` is byte-identical at
  `policy.rs:2422` and `is_separator(">")` is still `false`. **The SEGMENT-COUNT
  pins are green in both the SPLIT and the DISPLACED variant**, and a new pin
  asserts the same property for the new class.
* **Round 11's REDIRECTION CHANNEL** — the ancestor clause and the second field
  both sit against it. `: > <ENV>/alpha/pr-ledger.ndjson` and both near-miss
  controls did not move.

`Token::literal`, `Token::expansion`, `Token::word_splitting_flush`,
`Token::brace_splice`, `Token::redirection_unresolvable`,
`Segment::head_is_command_position`, `Segment::brace_spliced`,
`Segment::splice_can_produce_governed` and `Segment::redirection_targets` keep
their present meanings exactly. Rounds 5, 7, 8, 9 and 10's clauses are non-dead
with their controls at exit 0 and both `T-19-86` `!`-bodied rows at exit 0. Round
12's own — the interior scan, `containment_holds_…`, the `credential.helper`
clause with its five near-miss controls at the same identifier, and the ledger
KIND check at all four kinds — are green.

**`envelope_config_resolution` is 30/0 and NOT ONE of its thirty verdicts
moved**, checked after each code commit and again at the gate.
`POLICY_MIN_PRODUCTION_BYTES`, `HOOKS_MIN_PRODUCTION_BYTES`, the proportional
floor's value, the deep anchor and the sentinel counts are unchanged, and **no
second `#[cfg(test)]` sentinel was introduced in any of the three files.**

**`SECTION_ENVELOPE` shows ZERO diff lines and its headroom is UNSPENT**, at
`19-32`'s re-measurement of 211 whitespace tokens of an unraised 215 cap and a
widest line of 74 of 80. **`src/envelope/hooks.rs`, `src/envelope/mod.rs`,
`src/envelope/advisory.rs`, `src/scan.rs` and `src/config.rs` were NOT OPENED.**

---

## The gate

```text
BASE resolved by FILE CONTENT   07c551a2c6e106299d3cfba14746af14ce939899
  (git log --format=%H -n 1 -- …/19-32-SUMMARY.md)
cross-checked by MESSAGE PREFIX 07c551a2c6e106299d3cfba14746af14ce939899
  (git log --format=%H -n 1 --grep='^docs(19-32)')
                                 -> THE TWO ANCHORS AGREE

task3_severed = 0   (detected from the TREE by `const CANDIDATE_SCAN_WORK_CEILING`,
                     not from a marker file and not from a commit message)

rtk proxy cargo test --no-fail-fast
  48 result lines, 13 ignored
  1924 passed + 0 failed = 1924

19-32's recorded total                                              1914
new #[test] fns, counted from `git show` over this plan's commits:
    src/envelope/policy.rs
      the third word class: carries/splits-no-segment, is-a-path-class,
      per-command attribution                                          3
      the ancestor clause and its stop                                 1
      the four repaired claims                                         1
      the three-axis condition with no count                           1
    tests/envelope_word_set.rs
      the ceiling DRIVEN, the 50 000-slash row, the 800 KB row,
      the post-fix curve on three trees                                4
                                                                 -------
                                                                      10
  1914 + 10 = 1924      <-  EXACT, no disagreement

cargo build                            exit 0
cargo clippy -- -D warnings            exit 0   (`--tests` is NOT the gate)
```

**A red test RAN, so red→green leaves the total unchanged and every increase
comes ONLY from new `#[test]` fns. The identity holds exactly.**

### All NINETEEN `envelope_*` binaries RAN

```text
envelope_advisory            10/0     envelope_argv_deletion       20/0
envelope_callee_grammar      19/0     envelope_carrier_reach       39/0
envelope_command_position    18/0     envelope_config_resolution   30/0
envelope_control_carrier     37/0     envelope_credential           6/0
envelope_expansion_slots     32/0     envelope_hook_refusals        7/0
envelope_interior_path       41/0     envelope_literal_decision    43/0
envelope_pr_cap              11/0     envelope_reparsed_value      34/0
envelope_tracer               6/0     envelope_wiring              14/0
envelope_word_set            42/0     envelope_wrapper_bypass      13/0
envelope_wrapper_class       60/0
```

### The mechanical file gates, all checked with `|| exit 1`

```text
src/ across the plan   policy.rs, ledger.rs, cred.rs — and nothing else
hooks.rs / mod.rs / advisory.rs / scan.rs / config.rs   zero diff lines
Cargo.toml / Cargo.lock                                 unmoved (T-19-SC)
tests/ deleted lines                                    6, all comments or
                                                        the ONE authorised row
AR-19-13 rows in 19-SECURITY.md                         0
```

---

## Findings, and things found and NOT fixed

1. **The zero-deletions criterion was unsatisfiable again, in the same shape
   `19-32` reported.** The plan requires the authorised ancestor pin to MOVE in
   `tests/envelope_interior_path.rs` and requires the `SEPARATORS` citation
   corrected at three sites under `tests/` — **and a row that moves cannot be
   moved without deleting the line that carries it**, nor a citation corrected
   without changing the comment line that carries it. Git counts a changed line
   as one deletion. **This is reported rather than worked around.** The six
   deleted lines under `tests/` are: three `SEPARATORS` citation comments, two
   `///` doc lines of `simulated_protected`, and the ONE authorised ancestor row.
   Every one is inside the plan's own named exceptions and the mechanical gate
   confirms it. **No assertion was relaxed and no evidence row was edited.**
2. **The two authorised pin moves required an addition the plan did not name.**
   `every_fenced_unit_pin_…` runs against a test-local `simulated_protected`,
   which mirrors the production rule; the row could not move until that
   simulation carried the ancestor arm. Its signature is byte-identical and the
   arm moves exactly one of the thirty-two words it is driven over — verified by
   hand against every one before the edit.
3. **The `ledger.rs` API name still cannot be spelled in that comment, and the
   plan's own test for whether it may be is necessary but NOT sufficient.** The
   plan says to spell it *"if and only if `19-32`'s pin is a production-SLICE pin
   rather than a whole-file grep"*. Verified from the tree: it **is** a
   production-slice pin — **and its slice runs from the top of `ledger.rs` to the
   test sentinel, which COVERS the very comment the name would be written in.**
   Writing it turns that pin RED. The sufficient test is whether the region the
   pin guards contains the comment. The repair therefore states the honest
   reason — a fact about where the slice begins — rather than the vacuity claim
   the old sentence made, and this gap in the instruction is recorded rather than
   silently resolved.
4. **A FIFTH FLAKE, previously undocumented — found, measured PRE-EXISTING, and
   NOT fixed.**
   `the_t_19_119_replacement_takes_layer_2_as_well_and_that_is_measured_separately`
   in `tests/envelope_interior_path.rs`:

   ```text
   thread 'the_t_19_119_replacement_takes_layer_2_as_well_and_that_is_measured_separately'
   panicked at tests/envelope_interior_path.rs:1068:14:
   the request is written: Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }
   ```

   **The mechanism**: the `replaced` leg spawns `/bin/true`, which exits without
   ever reading stdin, and the parent's `write_all` races that exit and gets
   `EPIPE`. It is a FIXTURE race involving no guard code at all. **It is
   PRE-EXISTING, and that was MEASURED rather than argued**: the base commit
   `07c551a` was extracted with `git archive` outside the repository and built
   there — the working tree was never checked out, reset or stashed — and the
   same test fails **4 runs out of 4** under the same artificial CPU load, at
   `tests/envelope_interior_path.rs:1045`, with the identical panic. It fired
   twice here: once during a sweep that ran concurrently with a background
   `cargo build`, and twice more under sixteen deliberate busy loops. **It is
   green in every unloaded run** — five consecutive isolated runs and every
   full-suite run including the final gate. **Not fixed**, because the file is
   fenced beyond this plan's two authorised changes. **Absence from an unloaded
   run is not evidence it is fixed.**
5. **The four already-documented flakes did NOT fire** on any full-suite run of
   this plan — neither `driver_reattach`, nor the `envelope_tracer`
   `ExecutableFileBusy` race, nor the ETXTBSY race over the binary.
   `envelope_tracer` answered 6/0 throughout. **Absence is not evidence any of
   them is fixed**, which matters here because the here-string BINARY rows
   exercise `C-10`'s own seam directly and this round changed what the guard does
   about that seam. **That relation is claimed in NEITHER direction.**
6. **Two of this executor's own new mechanical pins were structurally
   incompatible with the WR-02 discipline on the first attempt**, and that is
   recorded rather than smoothed. An absence assertion over a stale form that the
   repair is REQUIRED to QUOTE can never pass. Both were rewritten to assert the
   CORRECTION's presence, or to slice the region the claim is actually about,
   rather than by deleting the quotation. It is a real constraint WR-02 puts on
   mechanical checks and it is stated in the tests themselves.
7. **One of this executor's own comment edits violated a stated prohibition and
   was caught by the control that exists for it.** Writing *"do not add a second
   `#[cfg(test)]` line"* into `ledger.rs`'s comment ADDED one, above the sentinel,
   truncating `19-32`'s production slice and turning
   `the_link_non_following_stat_variant_is_absent_from_ledger_rs_production_half`
   red on a POSITIVE CONTROL. Reworded; `ledger.rs` carries exactly one sentinel.
   **The pin failed on the right thing, loudly, and that is what it is for.**
8. **The ceiling's refusal message does not distinguish the two causes.** It
   reuses `ProtectedPath::EnvelopeDirectory`, because the plan mandates no new
   `ParkReason` and exactly two recovery lines. The identifier and the mechanism
   are right; the message names the protected path. Disclosed in the cost section
   and recorded here.

---

## Carried forward untouched

`AR-19-04`, `AR-19-05` and `T-19-23` are untouched — **not un-accepted, not
re-rated, not renumbered, not marked closed.** **`T-19-17r` is OUTSTANDING for
the EIGHTEENTH time**: `grep -cE '^\| AR-19-13 \|'` over `19-SECURITY.md` is
**0**, no Accepted-Risks-Log row was added, and **the word "accepted" is applied
to it nowhere** — not in a test name, a comment, this SUMMARY or the appended
subsection. **This plan did NOT accept it.**

`C-08`'s, `T-19-120`'s and `T-19-124`'s behavioural halves stay **UNMEASURED**
and are claimed in NEITHER direction. `C-11` … `C-15` keep control (e) with **no
`pr_cap_*` clamp**, and no rule, pin or doc claim was written for `.git/config`,
`.claude/settings.json`, `.git/info/exclude`, the run journal or
`~/.config/gsd-meta-manager/config.json`. `T-19-96`, `T-19-110` and `T-19-115`
are unchanged; the `glab --host` forge cell is unchanged, `FORGE_VALUE_OPTS`
keeps `--hostname`, and **`glab` is confirmed NOT INSTALLED** so no pin that
would skip was written. `T-19-74`'s core rows stay frozen; `park_refusal` was not
moved and `scan.rs` and `config.rs` were not opened. **No crate was added.**

---

## What remains uncovered

**`T-19-86` FIRST — OPEN at `high` by explicit user scoping decision**, entirely
unremediated; its rows are at exit 0 and its pins are green and UNMODIFIED. The
`!`-bodied ALIAS-BODY credential route measured by `19-32` is **its** route,
recorded in `cred.rs`'s NOT-COVERED list and **NOT folded into it**.

Then, **OPEN at `high`**: **`T-19-91`**, **`T-19-111`** (no rule), **`T-19-112`**
(narrowed further, not closed), **`T-19-116`** (NARROWED — the here-string route
closed; `rm -rf <binary-parent>` plus four original residues still open) and
**`T-19-121`**.

Then **`T-19-113`**, **`T-19-115`** (no rule, and none can be written — a tilde
needs the ENVIRONMENT and a glob the FILESYSTEM; its instances sit on axis 2),
and **`T-19-122`**, **`T-19-123`**, **`T-19-124`** (`medium`), **`T-19-125`** and
**`T-19-126`** (`low`) — **NARROWED or CORRECTED, and none closed by this plan's
claim.**

**NEW, registered by this plan: the ANCESTOR RESIDUE ABOVE THE ENVELOPE ROOT** —
`rm -rf /tmp` when the root is `/tmp/xyz` reaches the same nine carriers and no
rule is written for it. **Disclosed, UNACCEPTED**, named on axis 3, and handed to
no pin, schedule or version witness. **NEW, also registered: the fifth flake**,
measured pre-existing at the base commit.

Then **`T-19-96`**, **`T-19-110`**, **`T-19-74`**, **`T-19-84`**, **`T-19-85`**
and **`T-19-61` … `T-19-73`**.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

**`/gsd-secure-phase 19` is NOT cleared by this plan.**


---

## Self-Check: PASSED

```text
.planning/.../19-33-SUMMARY.md                               FOUND
src/envelope/policy.rs                                       FOUND
src/envelope/ledger.rs                                       FOUND
src/envelope/cred.rs                                         FOUND
tests/envelope_word_set.rs                                   FOUND
.planning/.../19-SECURITY.md (plan-19-33 record appended)    FOUND
.planning/.../deferred-items.md (plan-19-33 section)         FOUND
commit cb02cdf                                               FOUND
commit 9748141                                               FOUND
commit 47c0f1b                                               FOUND
commit 5e65ad1                                               FOUND
commit 0f66086                                               FOUND
```
