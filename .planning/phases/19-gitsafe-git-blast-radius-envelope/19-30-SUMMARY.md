---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 30
subsystem: envelope
tags: [security, gap-closure, corpus, interior-path, attachment-boundary, credential-helper, ledger-kind, T-19-119, T-19-120, T-19-121, T-19-116]
requires:
  - "audit 11 (19-SECURITY.md) — the plane is finished; the residues are one character short"
  - "19-29 (the round-11 rules) — its 1820-test post-state is this plan's baseline"
provides:
  - "tests/envelope_interior_path.rs — the ELEVENTH evidence file, 36 tests, 8 RED by design"
  - "the CONTROL-CARRIER axis at TWELVE classes with a fail-closed CONTROL_CARRIER_INTERIOR_PATH alphabet and the complement EXTENDED"
  - "the DESIGN ANSWER: `/`-anchored substrings of a LITERAL word, with containment stated over the NORMALISED STRING"
  - "T-19-119 driven END TO END twice against real git — a moved bare remote and a fired-then-reset PR cap"
  - "T-19-121's whole key-shape space measured; cred.rs:420-425 recorded FALSE"
  - "T-19-120's FIFO measured at exit 124/20.02 s with both controls"
  - "an exact handoff number for 19-31 to gate against: 1859 = 1820 + 39"
affects:
  - "19-31 — the rules and the honesty repairs, gated on this plan's recorded RED state"
  - "audit 12 — which judges whether anything closes; this plan claims nothing closed"
tech-stack:
  added: []
  patterns:
    - "a boundary stated over what the predicate READS, never over a list of attachment characters — a character list is a program-grammar enumeration and is D-08's defect one level over"
    - "containment of the NORMALISED STRING rather than of the index, because the normaliser strips leading `./`s before testing for `/`"
    - "a simulated rule, written test-local, so a derived post-fix verdict is COMPUTED rather than asserted by eye"
    - "a row whose PRE-fix verdict the fix changes is RECORDED, never asserted, in a file the next plan may only ADD to"
key-files:
  created:
    - tests/envelope_interior_path.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The option-attachment boundary is `/`-anchored substrings of a LITERAL word — not `=`, not a character list, not an option-spelling list"
  - "Containment is of the NORMALISED STRING, not of the index: the `./` strip can only ADD answers"
  - "Class 12 is fail-closed and class 7 the COMPLEMENT is extended — the sixth forced split, the second through the complement"
  - "Class 12 is NOT drawn in redirection-target position, and the doc says why rather than forcing symmetry"
  - "Every row whose PRE-fix verdict the fix changes is RECORDED here and ASSERTED only at its DERIVED post-fix verdict"
metrics:
  duration: "one session"
  completed: 2026-09-04
actuals:
  tokens: 91000
  tasks: 3
  commits: 3
status: complete
---

# Phase 19 Plan 30: The path that is not the word but is inside it — Summary

**The corpus can now fail on a path carried INSIDE a word, and it does: 9 rows RED across two files with zero `src/` hunks in all three commits.**

## FIRST: this plan closes nothing, and `/gsd-secure-phase 19` is NOT cleared

**`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`, `T-19-116`, `T-19-119` and
`T-19-121` all remain OPEN at `high`.** `T-19-120` is open at `medium`. This plan
wrote the corpus and the reproducers and STOPPED; `19-31` writes the rules and the
honesty repairs. **Whether `19-31` closes any of them is audit 12's judgement
rather than either plan's claim, and neither plan clears
`/gsd-secure-phase 19`.**

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" is written anywhere.

`git diff --numstat 1e56389..b6eaafa -- src/ tests/` was confirmed **EMPTY**
before the first measurement, and `git diff b6eaafa..77b3b90` touches only
`.planning/`, so every number here was measured against the tree audit 11
measured.

## The three commits, in order

| SHA | what | `src/` hunks |
|---|---|---|
| `88c892f` | `tests/envelope_interior_path.rs` — the ELEVENTH evidence file | **0** |
| `6911683` | the CONTROL-CARRIER axis, eleven classes → TWELVE | **0** |
| `0607fde` | the record: `19-SECURITY.md` + `deferred-items.md` | **0** |

`git diff --numstat HEAD~3..HEAD -- src/` is **empty**. `src/envelope/advisory.rs`
shows zero diff lines. Neither `Cargo.toml` nor `Cargo.lock` appears.

## THE GATE — and the arithmetic, stated and CHECKED

| | baseline (`19-29`) | post-state (`19-30`) |
|---|---|---|
| `passed + failed` | **1820** | **1859** |
| result lines | 46 | **47** |
| ignored | 13 | 13 |
| `envelope_*` binaries | 17 | **18** |
| failures | 0 | **9, all by design** |

**1859 − 1820 = 39, and 39 is exactly the number of new `#[test]` fns**, counted
from `git show`: **36** in `tests/envelope_interior_path.rs` and **3** added to
`tests/envelope_wrapper_class.rs`
(`the_control_carrier_axis_can_draw_a_carrier_path_inside_a_word_and_not_only_after_an_equals_sign`,
`a_command_carrying_a_carrier_path_inside_a_word_is_refused_after_19_31`,
`the_two_git_dir_spellings_are_pinned_by_name_on_opposite_sides_of_the_boundary`).
**A red test RAN, so the identity holds exactly and every increase came ONLY from
new tests.**

Command: `rtk proxy cargo test --no-fail-fast`, counted with `rtk proxy grep` over
a redirected log (D-34). `cargo build` and `cargo clippy -- -D warnings` both exit
0; `cargo clippy --tests` was NOT the gate.

### THE COMPLETE RED LIST, VERBATIM — `19-31`'s handoff contract

`tests/envelope_interior_path.rs` (28 passed, **8 failed**):

```
after_19_31_an_equals_attached_carrier_path_is_refused_over_both_protected_paths
after_19_31_an_attachment_that_is_not_an_equals_sign_is_reached_the_same_way
after_19_31_a_colon_attachment_inside_an_assignment_prefix_is_reached_too
after_19_31_an_assignment_prefix_naming_a_protected_value_is_refused_and_the_unprotected_twin_is_not
after_19_31_a_ledger_that_is_not_a_regular_file_is_refused_rather_than_read
after_19_31_policy_rs_5743s_own_dd_example_is_refused_in_the_direction_it_names
after_19_31_ordering_pin_a_is_the_three_way_git_dir_pin_with_the_one_character_restored
after_19_31_ordering_pin_b_across_segments_the_first_refusal_still_wins
```

`tests/envelope_wrapper_class.rs` (54 passed, **1 failed**):

```
a_command_carrying_a_carrier_path_inside_a_word_is_refused_after_19_31
```

**That is the whole allowed failing set.** Every other test in the suite is green.
**No row came up green where red was expected, and no row came up red where green
was expected** — every one of the nine is a row whose post-fix verdict was DERIVED
in writing from `19-31`'s mandated design.

### THE COMPLETE PERMITTED LIST — the other half of the contract

Asserted **exit 0 BEFORE AND AFTER**, so a rule that widened past the path set
turns these red rather than turning a driven run unusable:

```
the_cost_rows_carry_an_equals_and_a_slash_and_stay_permitted_before_and_after
the_near_miss_and_exact_path_controls_stay_permitted_before_and_after
the_verdict_preserving_spellings_from_rounds_10_and_11_stay_permitted
a_regular_ledger_just_under_the_bound_still_permits_and_still_counts_before_and_after
a_fresh_envelope_root_with_no_ledger_at_all_still_permits_before_and_after
the_ledger_kind_reachability_table_is_measured_with_its_four_silences_beside_it
the_verdict_preserving_control_carrier_alphabets_stay_permitted_before_and_after   (wrapper_class)
```

The individual rows behind them: `git --git-dir=/tmp/g status`,
`git -c aliasx.q="…" q`, `git log --format=%H`, `git commit --author='A <a@b.c>'`,
`sed s/x/y/ /tmp/f`, `curl https://github.com/o/r`,
`git push origin HEAD:refs/heads/gsd-auto/alpha/w` and its `:`-free twin,
`rg pr-ledger.ndjson src/`, `rm -f /tmp/pr-ledger.ndjson`,
`cat <ENVX>/alpha/pr-ledger.ndjson`, `cp /bin/true <BINPAR>/some-other-file`,
`ls <BINPAR>`, `PATH=/usr/bin:<BINPAR> mytool`, `cp /bin/true -t<BINPAR>`,
`GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x`, `R=/tmp/g`,
`mytool --opt=a=/tmp/g/x`, and the outside-the-path-set twin of **every** asserted
row in the SAME attachment (`dd if=/tmp/g/x of=/tmp/stolen`,
`tar --create --file /tmp/t -C/tmp/g`, `cp /bin/true -t/tmp/g`,
`chmod --reference=/tmp/g/x /tmp/y`, `rsync --temp-dir=/tmp/g …`,
`PATH=/usr/bin:/tmp/g mytool`), plus rounds 10 and 11's whole verdict-preserving
half — the tilde, glob, brace, expansion-borne, relative and outside-the-root
rows, `git config --get core.hooksPath` and
`git -c alias.p='!git push --force origin main' p` (`T-19-86`, which may not move).

### The per-binary counts — ALL EIGHTEEN `envelope_*` binaries RAN

```
envelope_advisory.rs          ok      10 passed  0 failed
envelope_argv_deletion.rs     ok      20 passed  0 failed
envelope_callee_grammar.rs    ok      19 passed  0 failed
envelope_carrier_reach.rs     ok      39 passed  0 failed
envelope_command_position.rs  ok      18 passed  0 failed
envelope_config_resolution.rs ok      30 passed  0 failed   <- the 30/0 pin, unchanged
envelope_control_carrier.rs   ok      37 passed  0 failed
envelope_credential.rs        ok       6 passed  0 failed
envelope_expansion_slots.rs   ok      32 passed  0 failed
envelope_hook_refusals.rs     ok       7 passed  0 failed
envelope_interior_path.rs     FAILED  28 passed  8 failed   <- NEW, this plan's own file
envelope_literal_decision.rs  ok      43 passed  0 failed
envelope_pr_cap.rs            ok      11 passed  0 failed
envelope_reparsed_value.rs    ok      34 passed  0 failed
envelope_tracer.rs            ok       6 passed  0 failed
envelope_wiring.rs            ok      14 passed  0 failed
envelope_wrapper_bypass.rs    ok      13 passed  0 failed
envelope_wrapper_class.rs     FAILED  54 passed  1 failed
                                     430 tests over 18 binaries
```

**A run reporting seventeen would be a run in which this plan's own evidence file
did not execute. It reports eighteen.**

**None of the four documented flakes fired** — not the two `driver_reattach`
failures, not `envelope_tracer`'s `ExecutableFileBusy` stub-write race, and not
the ETXTBSY race over the binary, which this plan exercises directly through
`replace_through_of`'s twenty-attempt retry-then-rename fallback. **Absence is not
evidence any of them is fixed**, and nothing here was done to them.

## THE DESIGN ANSWER — where the option-attachment boundary is

`lexical_absolute_components` (`policy.rs:5457-5476`) opens with
`if !word.starts_with('/') { return None; }`, and BOTH halves of
`protected_carrier_named`'s path set are built on it. **The whole gap is that the
WORD must start with `/`.**

| Candidate boundary | Reaches | Misses |
|---|---|---|
| split at the FIRST `=` | `of=/p`, `--git-dir=/p` | `--opt=a=/p`; `-C/p`, `-t/p` (ATTACHED SHORT OPTIONS, no `=`); `host:/p` |
| split at EVERY `=` | the above plus `--opt=a=/p` | the attached short options and the `:` |
| a LIST of attachment characters | most spellings | whatever the next program uses — **and it is a PROGRAM-GRAMMAR ENUMERATION, D-08's defect one level over** |
| **`/`-ANCHORED SUBSTRINGS of a LITERAL word** | **all of them, every attachment character, and none** | only what the four existing conditions already exclude |

**A path CAN appear after more than one attachment character and after none, and
is reached either way** — `--opt=a=/p` and `-C/p` yield the same candidate as
`of=/p` does.

**CONTAINMENT, STATED PRECISELY.** It is NOT *"`i == 0` is today's rule"*:
`lexical_absolute_components` strips leading `./`s **before** testing
`starts_with('/')`, so `.//abs/p` is accepted today and `./abs` is not. The exact
statement is that **for every word today's rule answers `Some(v)` for, the string
it actually NORMALISES begins with `/`, is therefore itself one of the candidates,
and re-normalises to exactly `v`.** The `./` strip can only ADD answers and never
remove one. **No refusal that exists today can be lost.** Re-derived mechanically
in `containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte`, with
the ADD direction proved as its converse control.

**THE COST, with candidate component lists written out:**

```text
--author=A <a@b.c>                 no `/` in the word              -> no candidate
--format=%H                        no `/`                          -> no candidate
sed s/x/y/                         [x, y], [y], []                 -> shorter than any envelope dir
https://github.com/o/r             [github.com, o, r], [o, r], [r] -> wrong FIRST component
HEAD:refs/heads/gsd-auto/alpha/w   [heads, gsd-auto, alpha, w], …  -> wrong FIRST component
git --git-dir=/tmp/g status        [tmp, g]                        -> not under <root>/<alias>
FOO=/tmp/x cmd                     [tmp, x]                        -> not protected: PERMITTED
rg pr-ledger.ndjson src/           `src/` -> the EMPTY component list
```

The mechanical reason is `word_is_within`'s `dir.is_empty() || word.len() <
dir.len()` early return over COMPONENT VECTORS: a short suffix cannot match a deep
directory at all. `git --git-dir=/tmp/g status` alone is pinned PERMITTED in FOUR
files and named in FIVE `policy.rs` doc sites, **so a rule that turned it red
would turn five files red at once.**

## THE ATTACHMENT SWEEP, with the `bash` reach column and the twin beside each row

| spelling | guard | space-separated twin | real `bash` reach |
|---|---|---|---|
| `dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson` | 0 | `cp /dev/null <ledger>` → 2 | REACHES (truncated) |
| `tar --directory=<ENV>/alpha …` | 0 | `tar --directory <ENV>/alpha …` → 2 | REACHES (`tar -tf` lists it) |
| `cp --target-directory=<ENV>/alpha /bin/true` | 0 | → 2 | REACHES (`true` appeared) |
| `rsync --temp-dir=<ENV>/alpha …` | 0 | → 2 | REACHES (accepted, used) |
| `chmod --reference=<ENV>/alpha/pr-ledger.ndjson F` | 0 | → 2 | REACHES (F took 0644) — **a READ** |
| `git --git-dir=<ENV>/alpha push --force …` | 2 `force_push_blocked` | → 2 `envelope_assertion_failed` | REACHES (git names it back) |
| **`tar -C<ENV>/alpha …`** — no `=` | 0 | → 2 | **REACHES** |
| **`cp /bin/true -t<ENV>/alpha`** — no `=` | 0 | → 2 | **REACHES** |
| **`PATH=/usr/bin:<ENV>/alpha mytool`** — a `:` | 0 | — | **REACHES (bash EXECS out of it)** |
| `GSD_MM_ENVELOPE_ROOT=<ENV>/alpha gh pr create` | 0 | — | value delivered |
| `dd if=/bin/true of=<BINARY>` | 0 | `cp /bin/true <BINARY>` → 2 | REACHES |
| `tar --directory=<BINARY> …` | 0 | — | REACHES |
| `python3 --out=<ENV>/alpha/x` | 0 | — | **DOES NOT** — `unknown option --out=…` |
| `mytool --opt=a=<ENV>/alpha/…` — TWO `=` | 0 | — | **DOES NOT** — no such grammar found |
| `ssh -oControlPath=<ENV>/alpha/s host` | 0 | — | **DOES NOT** — connection never opened |
| `rsync host:<ENV>/alpha /tmp/x` | 0 | — | **DOES NOT** — no remote |

The four unreached spellings are **RECORDED, never asserted**: a spelling the
guard permits but the shell does not reach is not a bypass.

**The two genuinely-reaching TWO-`=` spellings that WERE found are both already
refused today by OTHER clauses**, so neither discriminates on this axis and both
are recorded with their overlap stated:
`GIT_CONFIG_PARAMETERS='core.excludesFile=<ENV>/alpha/excl' git status` is exit 2
`hook_bypass_blocked` **and so is its unprotected control, at the same
identifier**; `git -c alias.q=--git-dir=<ENV>/alpha q` is exit 2
`envelope_assertion_failed` from round 9's re-parse clause, and real git refuses
to run it at all (`alias 'q' changes environment variables`).

## `T-19-119` DRIVEN END TO END, TWICE

```text
CONTROL  real binary                     push --force -> REFUSED,   main 0bd5d44 UNMOVED
GUARD    cp /bin/true <BINARY>                        -> exit 2
GUARD    dd if=/bin/true of=<BINARY>                  -> exit 0     <- the finding, two lines
LEG B    binary replaced through `of=`   push --force -> COMPLETED, main 0bd5d44 -> 6efde46 MOVED
LAYER 2  <replaced> envelope guard alpha              -> exit 0     (real binary: exit 2)
CONTROL  binary restored, remote rewound push --force -> REFUSED,   main 0bd5d44 UNMOVED
```

```text
call 1  gh pr create --title x  -> exit 0 (permit),        1 ledger line
call 2..4                       -> exit 2 pr_cap_exceeded, 4 lines (412 bytes)
GUARD   cp /dev/null <ledger>   -> exit 2   <- the TWIN
GUARD   dd if=/dev/null of=<ledger> -> exit 0; truncated to 0 bytes
call 5  gh pr create --title x  -> exit 0 PERMITTED AGAIN, 1 line
```

The bare-remote fixture was **REBUILT**, not cited; the cap drive used **ONE
persistent root**; every other guard row used a **fresh `GSD_MM_ENVELOPE_ROOT`**
with the whole root walked afterwards, and the walk is proved non-blind by
`the_positive_control_proves_the_walk_can_see_a_ledger_line_at_all`.

## THE RESTATED RESIDUE CONDITION, with NO count

> This predicate reads every `/`-ANCHORED SUBSTRING of every LITERAL word, in
> either word class, against both paths. It is silent about a word the SHELL MAY
> REWRITE; about a word whose text carries NO ABSOLUTE PATH ANYWHERE; and about a
> word that reaches a protected path ONLY THROUGH A LINK. The spellings are
> INSTANCES and are not counted.

**The one-character correction: *"a word that IS NOT ABSOLUTE"* becomes *"a word
whose text carries no absolute path anywhere in it."*** Handed to **no** pin, **no**
schedule and **no** version witness.

## WHAT CLOSING `T-19-119` DOES AND DOES NOT DO FOR `T-19-116`

**Does:** every option-attached spelling of the binary path becomes refused,
including the one that moved a bare remote's `main`.

**Does not** — all four re-measured at exit 0:

```text
exit=0  cp /bin/true $(command -v gsd-meta-manager)          <- expansion-borne
exit=0  cp /bin/true ~/.cargo/bin/gsd-meta-manager           <- tilde
exit=0  cd <binary-parent> && cp /bin/true gsd-meta-manager  <- relative
        a PATH symlink whose target current_exe() reports    <- link
```

**None is an option attachment. `T-19-116` stays OPEN at `high`.**

## THE PROCESS FINDING — stated plainly, written BESIDE

`19-28-SUMMARY.md:381-383` recorded the `=`-joined spelling's differing verdict
**and named its mechanism exactly right**; `19-29-SUMMARY.md:641-644` carried it
forward as *"Not a defect."* **The observation was correct and the classification
was wrong.** An operand whose verdict changes when you delete one space is not a
curiosity about identifiers; it is a word class the rule cannot see. **A
recorded-and-misclassified observation is how this survived a round that had
already found it — an observation you cannot derive is recorded as UNRESOLVED, not
classified.** Neither `19-28`'s nor `19-29`'s subsection was edited; the correction
is written beside them in this plan's own section, the discipline audit 11 used for
its own two corrections.

## `T-19-121` — the key-shape space, and the claim recorded FALSE

`cred.rs:416-425`, verbatim:

> * **a later `-c credential.helper=<something>` on the same command line, which
>   OVERRIDES the reset and brings the secret back.** … the bound is stated rather
>   than assumed: **that spelling is ARGV-VISIBLE and is already governed** by
>   [`super::policy::scan_leading`]'s leading-option region and layer 2's whole
>   grammar — unlike every write spelling, which is not.

**ARGV-VISIBLE it is. GOVERNED it is not.** Measured, with the **no-`-c` CONTROL
driven FIRST** under the envelope's full posture including `19-29`'s injected
empty-helper pair:

```text
                                                   real git             guard
CONTROL  no `-c` at all                            exit 128, ABSENT     —
-c credential.helper=store                         secret PRESENT       exit 0
-c CREDENTIAL.HELPER=store                         secret PRESENT       exit 0
-c Credential.Helper=store                         secret PRESENT       exit 0
-c credential.https://github.com.helper=store      secret PRESENT       exit 0   <- URL-SCOPED
-c credentialx.helper=store                        secret ABSENT        exit 0
-c notcredential.helper=store                      secret ABSENT        exit 0
-c credential.helperx=store                        secret ABSENT        exit 0
--config-env=credential.helper=<VAR>               secret PRESENT       exit 0
GIT_CONFIG_PARAMETERS="'credential.helper=store'"  secret PRESENT       exit 2 hook_bypass_blocked
```

The secret is recorded PRESENT/ABSENT and never transcribed. The by-name deny
covers `core.hooksPath` (measured `hook_bypass_blocked`) and round 8's clause
covers `include.path` (measured `envelope_assertion_failed`); **nothing covers
`credential.helper`, measured exit 0 on four surfaces.** This is
`T-19-84`/`T-19-107`/`T-19-109`/`T-19-115`'s shape a **SIXTH** time, and the
correction is required **whether or not any rule lands**. **The URL-scoped and
`--config-env` shapes are recorded as what a by-name clause would NOT reach**, so
`19-31` can state its partiality rather than discover it. **No rule is asserted:**
the four guard rows are RECORDED, because `19-31`'s clause is SEVERABLE and this
file is one `19-31` may only ADD to.

## `T-19-120` — the FIFO, with both controls

```text
FIFO ledger (stat size 0)          gh pr create --title x -> exit 124 after 20.02 s
8 366 000-byte regular ledger                             -> exit 0 in 0.42 s, STILL COUNTING
FRESH root, no ledger at all                              -> exit 0 in 0.01 s, exactly 1 line
```

**The fix belongs INSIDE the same `Ok` arm.** A fresh root takes the `Err`
fall-through, and a kind check outside it would refuse every first forge call in
every fresh root; the fresh-root row is asserted exit 0 BEFORE AND AFTER, with the
comment saying that a red there means the check left the `Ok` arm. The
reachability table shows the absolute-literal `mkfifo` at exit 2 and the tilde,
glob, brace and expansion-borne spellings at exit 0. **The behavioural half —
what the agent CLI does with a hook that never returns — is UNMEASURED and claimed
in NEITHER direction.**

The row is driven through the PRODUCT binary under a hard `timeout` rather than
in-process, because a blocking read would hang the whole test binary; it costs
12 s while red and returns immediately once `19-31` lands.

## `policy.rs:5743` recorded FALSE

`dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen` is **exit 0** — the doc argues
the read over-refusal is unavoidable using a spelling the guard refuses **neither
way**. Both repairs recorded; the correction is required regardless.

## THE FENCED-FILE ENUMERATION — re-derived, and the answer is ZERO

All three greps were run and their output is recorded verbatim in the
`19-SECURITY.md` section. Grep 1 returns **5 hits, every one a `//` or `///` DOC
line**. Grep 2 returns 8 hits: five `refs/heads/gsd-auto/{}/…` refspecs (candidates
`[heads, gsd-auto, <alias>, …]` — **wrong first component**), one `/proc/{}/stat`,
and `envelope_config_resolution.rs:1825/:1832`'s
`includeIf.gitdir:{repo}/.path={evil}` where `{repo}` is a **temp git repo** and
the row is a **real-git probe, not a guard row**. Grep 3 shows that every
`=`-attached absolute path in the whole corpus is `/tmp/evil.cfg`, `/dev/null`,
`/tmp/nohooks`, `/tmp/g`, `/tmp/e`, `/tmp/x`, `/tmp/s`, `/tmp/fresh`,
`/tmp/nowhere`, `/tmp/evil`, `/x`, `/ENV_WINS`, `/CLI_WINS`, `/PARAM_WINS`,
`/ALIAS_WINS`, `/INCLUDE_WINS`, `/PARAM_INCLUDE_WINS`, `/SHOULD_NOT_WIN` or
`/nonexistent-hooks-dir` — **not one names an envelope root and not one equals a
binary.**

**ZERO assertions move**, and it is proved mechanically rather than by reading:
`every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule`
runs the mandated design over nine `policy.rs` unit pins and twenty fenced corpus
words, with eight positive controls so the absences are not vacuous. Both named
pins re-derived by hand and mechanically:
`word_is_within("/tmp/envroot/alpha/../other/x")` stays `false` because `..`
collapses PER CANDIDATE, and `word_is_within("./alpha/pr-ledger.ndjson")` stays
`false` because two components is shorter than three.

**ONE artefact moves and it is a COMMENT: `policy.rs:9719`'s stated reason for the
`./alpha/…` row goes stale. Named as `19-31`'s WR-02 correction; not edited.**

## THE AXIS — twelve classes, with the complement extended

```text
CONTROL_CARRIER_CASES            61 -> 84   (+12 interior, +11 ordinary)
CONTROL_CARRIER_SLOTS            12 -> 13
MIN_CONTROL_CARRIER_CLASSES      11 -> 12
MIN_CONTROL_CARRIER_ORDINARY_OPERANDS  13 -> 24
MIN_CONTROL_CARRIER_INTERIOR_PATH        —  -> 12
class 7 count                    13 -> 24   (NAIVE without the extension: 36)
class 12 count                    —  -> 12
every other class count          UNCHANGED (11, 4, 3, 3, 2, 6, 5, 5, 5, 5)
MIN_CONFIG_RESOLUTION_CLASSES    6, unchanged
```

**Class 7's fall from a naive 36 to 24 is the arithmetic proof the complement
extension took effect** — twelve entries that would otherwise have sat in the
INVARIANCE arm asserting PERMITTED a row `19-31` refuses. That is `19-18`'s `{v}>`
blocker, `19-20`'s split, `19-22`'s, `19-24`'s and `19-28`'s, **a SIXTH time and
the SECOND through the complement.** Class 5 was refined for the same reason round
11 refined it for the tilde, and stays at 2 because both its entries carry no `/`
at all.

**The three fences:**

* **NO-INTERIOR-PATH (new)** — counts **12** interior carrier words across the
  thirteen alphabets, **3 of them attached by something OTHER than `=`**
  (`-C…`, `-t…`, `PATH=/usr/bin:…`). Its failure message names the `=`-only trap
  explicitly and says the correct response is to ADD a spelling.
* **NO-PROGRAM-NAMES** — extended to the new alphabet, carried by `cp`, `chmod`,
  `rsync` and `mytool`.
* **PATH-PREFIX-NOT-BASENAME** — extended with the six shapes a short candidate can
  never match, plus a positive control.

`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` stays
green. Disjointness is re-proved over all twelve with section 17's
reachability-not-text limitation intact. The four earlier axes, their predicates,
their degenerate-proofing and their floors are byte-identical, and
`tests/envelope_wrapper_class.rs:5133`'s `--git-dir=/tmp/g` `GitGlobalOption`
entry did not move.

## The two ORDERING pins, at deliberately different identifiers

**PIN A — the three-way `--git-dir` pin, `19-28`'s own observation with the one
character restored:**

```text
git --git-dir=<ENV>/alpha push --force origin main   force_push_blocked      -> ASSERTED envelope_assertion_failed  RED
git --git-dir <ENV>/alpha push --force origin main   envelope_assertion_failed -> ASSERTED unchanged                GREEN
git --git-dir=/tmp/other  push --force origin main   force_push_blocked      -> ASSERTED unchanged                  GREEN
```

**PIN B — across segments, where order decides the identifier:**

```text
dd if=/bin/true of=<ENV>/alpha/x && git push --force origin main   -> ASSERTED envelope_assertion_failed  RED
git push --force origin main && dd if=/bin/true of=<ENV>/alpha/x   -> ASSERTED force_push_blocked         GREEN
dd if=/bin/true of=/tmp/other/x && git push --force origin main    -> ASSERTED force_push_blocked         GREEN
```

**All orderings were measured before any was written.** If PIN B's second row ever
answered `envelope_assertion_failed`, the clause would be running in a SECOND PASS
over the segments — a finding to report, not a row to relax.

## The rows RECORDED and never asserted, proved mechanically

`the_unruled_rows_are_recorded_and_never_asserted` drives seventeen `record_only`
rows: `T-19-116`'s three residues; `C-11` … `C-15`; `C-08`'s behavioural half;
`T-19-121`'s URL-scoped, `--config-env` and four guard rows;
`GIT_CONFIG_PARAMETERS`; the two already-refused two-`=` spellings; the four
unreached attachment spellings; and `glab`. The three over-refusal instances plus
the `?` variant are recorded separately.
`no_repo_side_or_unruled_row_is_asserted_and_this_file_says_so_mechanically` reads
the file's own text over fifteen fragments and proves not one of them is inside
`refuses`, `refuses_carrier`, `permits` or `permits_carrier` — with two positive
controls first, so the absence is not vacuous.

## Mechanism pins and byte floors, re-measured

`SEPARATORS` byte-identical at `policy.rs:2297`, ONE commit in the whole phase
(`84a9b05`); `is_separator(">")`, `("<")` and **`("=")`** all `false`, with the
real separators as the positive control; `segment.tokens` byte-identical, with the
SEGMENT-COUNT pins green over both the `>/dev/null` and the `x2>` rows and
asserting the count **and** the exact token vector so both the SPLIT and the
DISPLACED variant are caught; round 5's literalness bit non-vacuous, read from the
real tokenizer; round 6's deletion model, its over-deletion control, the
no-target row and the heredoc row; round 7's four callee-grammar directions;
round 8's confinement clause with both `--signed no` controls; round 9's re-parse
clause with `aliasx.`/`notalias.` and the `T-19-86` `!`-bodied row at exit 0; round
10's nine envelope-root operand rows plus `env -u` both ways; round 11's
redirection-target and exact-path clauses with both near-miss controls permitted;
`envelope_config_resolution` at **30/0**. The byte floors, the proportional floor,
the deep anchor and the one-`#[cfg(test)]`-sentinel count are untouched and the
floor comment stays as `19-29` corrected it.

## `SECTION_ENVELOPE` — re-measured, NOT edited

**211 whitespace tokens of an UNRAISED 215 cap. Widest line 74 of 80.** The first
`Guaranteed` clause is still TRUE and `T-19-121` does not falsify it: *"As
started"* states what the envelope ESTABLISHES and stops. The `Not guaranteed`
half's generalisation to *"the files and the binary this envelope runs on"*
already covers this round's route, so **no new disclosure is owed and none was
written**. `src/envelope/advisory.rs` shows **zero diff lines**, and `19-31` is
prohibited from opening it too.

## Deviations from plan

### 1. [Rule 3 — blocking] `-cf` and a trailing operand trip `NestedPayload`, so four spellings were re-written

- **Found during:** Task 2, on the axis's own reachability assertion (A).
- **Issue:** `resolve_program_with_head`'s rule 6 (`policy.rs:4558-4566`) treats
  **any short option containing `c` followed by another word** as the shell's
  `-c` spelling and answers `NestedPayload`. `tar --directory=<ENV>/alpha -cf
  /tmp/t .` therefore resolves `NestedPayload { index: 3 }`, which
  `the_control_carrier_axis_is_new_because_no_existing_axis_reaches_a_command_with_no_governed_program`
  correctly refuses to accept. **Worse, it is latent flakiness for the
  guard-driven rows**: `cp -t<TempDir>/alpha /bin/true` would have taken the same
  branch whenever the random temporary path happened to contain a lowercase `c`.
- **Fix:** four alphabet entries and their twins re-written to spellings that
  cannot take that branch — `--create --file` instead of `-cf`, and the attached
  short option moved to the LAST word of the segment (`cp /bin/true -t<dir>`,
  `tar --create --file /tmp/t -C<dir>`), where rule 6's `segment.len() > index + 1`
  is false. **Both were re-probed under real `bash` and both still REACH the
  directory**, so the sweep's evidence is unchanged.
- **Files modified:** `tests/envelope_wrapper_class.rs` (Task 2's own new entries;
  no pre-existing entry moved).
- **Commit:** `6911683`

### 2. [Honesty] Nine `-` lines under `tests/`, and every one is a floor that ROSE

- **Found during:** Task 2's verify.
- **Issue:** the plan's `<done>` asks for **zero deletions** under `tests/`, which
  cannot hold alongside its own mandate to RAISE `MIN_CONTROL_CARRIER_CLASSES`,
  `CONTROL_CARRIER_CASES`, `CONTROL_CARRIER_SLOTS` and
  `MIN_CONTROL_CARRIER_ORDINARY_OPERANDS` and to re-derive
  `CONTROL_CARRIER_CLASS_COUNTS`. A constant whose value rises is one deleted line.
- **Disposition:** **not fixed, reported.** The nine deleted lines are four doc
  lines re-emitted with a widened statement and **five floors that ROSE** (61→84,
  12→13, 11→12, 13→24, and class 7's count 13→24). **No alphabet entry, predicate
  clause, assertion or test fn was removed, reworded away or narrowed**, which is
  what the plan's prohibition actually forbids. The mechanical proxy is
  over-strict; the semantic rule is met.
- **Commit:** `6911683`

### 3. [Honesty] Two rows had to be RECORDED rather than asserted, because `19-31` may only ADD to this file

- **Found during:** Task 1, on the first run of the evidence file.
- **Issue:** the bare-remote and cap-reset drives each assert the PRE-fix guard
  verdict of the `=`-joined spelling (exit 0). **`19-31` turns that verdict over**,
  and its own prohibition limits it to ADDITIONS in
  `tests/envelope_interior_path.rs` — so an assertion at the pre-fix verdict would
  land permanently red with no permitted repair. The same applies to the four
  `-c credential.helper=` guard rows and to the `CRED_SOURCE`/`POLICY_SOURCE`
  claim quotations, which `19-31` REPAIRS.
- **Fix:** all of them converted to `println!`/`record_only`, each with the reason
  stated in place. The space-separated twins (`cp /bin/true <BINARY>`,
  `cp /dev/null <ledger>`) are **stable in both trees** and stay asserted, so both
  drives still have a discriminating control; the `=`-joined verdicts are asserted
  at their DERIVED post-fix verdict in section 1 instead.
- **Files modified:** `tests/envelope_interior_path.rs`
- **Commit:** `88c892f`

### 4. [Finding] The end-to-end fixture's binary is not `current_exe()`, and the guard is silent about it

- **Found during:** Task 1, first run.
- **Issue:** `cp /bin/true <fixture-copy-of-the-binary>` came back **exit 0**, not
  exit 2. `word_is_exactly` compares against the binary `guard` was HANDED —
  `current_exe()`, which under `cargo test` is the test binary — and the fixture's
  private copy is a different absolute path.
- **Disposition:** **correct behaviour, and it is the fail-open the predicate's own
  signature already states.** The twin pair is now driven over `current_exe()` and
  the fixture copy's verdicts are RECORDED beside it with the reason. **It is also
  what lets the layer-3 legs run at all** — the corpus's own protection does not
  block the fixture from replacing its own copy.
- **Commit:** `88c892f`

### 5. [Finding] Two planning-time claims corrected by measurement

- **`GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x` is RECORDED, not
  pinned.** The plan says it is *"pinned PERMITTED at
  `tests/envelope_control_carrier.rs:1106`"*; it is a `record_only` row at
  `:1105-1107` (`E-01`'s inert-prefix measurement). This plan's evidence file
  ASSERTS it, because after `19-31` its verdict is load-bearing.
- **`T-19-104`'s `GIT_CONFIG_PARAMETERS` carrier is exit 2, not exit 0.** The
  plan's RECORDED-rows list calls it *"open at exit 0"*. At the guard it is
  **exit 2 `hook_bypass_blocked`** — the key is in `ENVELOPE_ENV_KEYS`. Real git
  DOES resolve the helper from it, so the reach is real and the guard already
  refuses the carrier. Recorded as measured in both documents.

### 6. [Finding] The `?` and `#` spellings of the same URL get different answers

- The plan asks for a URL-shaped over-refusal instance *"after a `?`/`#`"*.
  **They are not the same row.** `?` is a pathname-expansion metacharacter, so
  `Token.literal` is FALSE and the interior scan never runs on the word: its
  derived post-fix verdict is **exit 0, unchanged**. Only the `#` spelling is an
  over-refusal. Both are RECORDED, with the difference stated — it is why the
  surface must be described over what the predicate READS rather than over what a
  URL looks like.

## Self-Check: PASSED

```
FOUND: tests/envelope_interior_path.rs
FOUND: tests/envelope_wrapper_class.rs
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
FOUND: 88c892f  test(19-30): the ELEVENTH evidence file
FOUND: 6911683  test(19-30): the CONTROL-CARRIER axis from eleven classes to TWELVE
FOUND: 0607fde  docs(19-30): the round-12 record
git diff --numstat HEAD~3..HEAD -- src/                        -> EMPTY
git diff --numstat HEAD~3..HEAD -- src/envelope/advisory.rs    -> EMPTY
grep -cE '^\| AR-19-13 \|' 19-SECURITY.md                      -> 0
Cargo.toml / Cargo.lock in the diff                            -> ABSENT
```

## Known Stubs

None. No stub, placeholder or hardcoded empty value was written; every row in
both files is either driven against the built binary or computed from the
mandated design.

## What remains uncovered

**`T-19-86` FIRST — OPEN at `high` by explicit user scoping decision.** Not fixed,
not narrowed, not re-scoped, not re-classified; its registered rows stay at exit 0
and its pins green and UNMODIFIED, and **`T-19-111` is kept OUT of it**.

Then, open at `high`: **`T-19-91`** (three arms unweakened); **`T-19-111`** (no
rule, rows recorded in neither direction, `19-27`'s five-site attribution
correction unchanged); **`T-19-112`** (narrowed, not closed — `T-19-119` is a
FOURTH route to its cap reset); **`T-19-116`** (narrowed further by `T-19-119`'s
closure, NOT closed, four residues at exit 0); **`T-19-119`**; **`T-19-121`**.

Then: **`T-19-113`** and **`T-19-115`** (no rule; the condition is RESTATED, not
closed); **`T-19-120`** at `medium`; **`T-19-96`**, **`T-19-110`**, **`T-19-74`**
(core rows frozen); **`T-19-84`**, **`T-19-85`** and **`T-19-61` … `T-19-73`**
(open and unaccepted by explicit user decision; `scan.rs` and `config.rs` were not
opened). **`C-08`'s and `T-19-120`'s behavioural halves stay UNMEASURED. `C-11` …
`C-15` keep control (e) with no rule and no `pr_cap_*` clamp. The `glab --host`
forge cell is unfixed, `FORGE_VALUE_OPTS` keeps `--hostname`, and `glab` is
confirmed NOT INSTALLED.**

**`T-19-17r` is OUTSTANDING for the FOURTEENTH time.** This plan did NOT accept it,
added no Accepted-Risks-Log row, created no `AR-19-13`, and applied the word
"accepted" to it nowhere. `AR-19-04` and `AR-19-05` were not un-accepted, re-rated
or renumbered, and **`T-19-23` was not marked closed**.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**
