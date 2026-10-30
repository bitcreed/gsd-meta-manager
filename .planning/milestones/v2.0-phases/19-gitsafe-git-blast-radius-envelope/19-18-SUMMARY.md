---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 18
subsystem: envelope
tags: [security, corpus, red, tokenizer, redirection, line-continuation]
status: complete
requires:
  - 19-17 (round 5's inverted literalness rule, and its 1584-test gate)
  - "19-SECURITY.md audit 5 (T-19-97, T-19-98, T-19-99)"
provides:
  - "tests/envelope_argv_deletion.rs — the FIFTH evidence file, 17 #[test] fns, 4 RED"
  - "tests/envelope_wrapper_class.rs section 14 — DELETION_CLASSES, a SECOND named axis, 4 #[test] fns, 1 RED"
  - "the exact handoff number 19-19 gates against: passed + failed = 1605"
  - "the complete RED name list, which is 19-19's handoff contract"
affects:
  - 19-19 (writes the rule; gated on this plan's recorded RED state)
tech-stack:
  added: []
  patterns:
    - "corpus written and observed RED BEFORE the rule — the third consecutive round to split the plan for this reason"
    - "a SECOND named class axis standing beside the untouched assembly classes rather than smuggled into them"
    - "floors as EXACT equalities derived from stated generation arithmetic, not >= guesses"
    - "argv-printing bash shims writing to a side file, so a >/dev/null row cannot swallow its own evidence"
key-files:
  created:
    - tests/envelope_argv_deletion.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The four FORGE rows assert a RESTORED COUNT (exit 0 with exactly one ledger line), not a refusal — the SAFE-06 cap is bypassed UNCOUNTED rather than exceeded, and asserting exit 2 would trade a correct count for a false positive on ordinary forge syntax"
  - "Three rows whose post-fix verdict 19-18 cannot derive are RECORDED and never asserted: >$F, >*.log and {v}>"
  - "{v}>/tmp/o is kept out of DISPLACING_REDIRECTIONS because every entry there is asserted verdict-PRESERVING and 19-19 does not model a {name} fd prefix"
  - "The generative property runs its PERMITTED arm first, so today's failing output is itself evidence the permitted half passed"
  - "T-19-17r's bookkeeping gap is recorded OUTSTANDING; no AR-19-13 row added and no acceptance made"
metrics:
  duration: "~75 min"
  completed: 2026-08-29
actuals:
  tokens: 71000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 18: The Deletion-Axis Corpus, Written RED Summary

The corpus for the other half of the boundary — a word must not only be written
literally, it must SURVIVE into argv — written and observed failing before a line
of the rule exists.

## `/gsd-secure-phase 19` is NOT cleared, and this plan closes NOTHING

Stated first, as the plan requires.

**`T-19-86` remains OPEN at `high`** by explicit user scoping decision. All four
of its rows still exit 0 and
`the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted` is
green and unmodified.

**`T-19-91` remains OPEN at `high`** with three arms unweakened: `git reflog $S`,
`git reflog show $S` and `git symbolic-ref $S` at exit 0 with no second carrier,
and the bare `git push $REF` cwd-dependent and permitted in-namespace — a THIRD
arm and **not** "already fails closed".

**`T-19-97`, `T-19-98` and `T-19-99` all stay OPEN.** `T-19-99` is closed only
when `19-19`'s rule is certified by the corpus written here, because a corpus is
evidence about a control and there is no control yet.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` are both sub-classes of it and both remain open, so neither this plan
nor `19-19` clears `/gsd-secure-phase 19`.

## The three commits, each with ZERO `src/` hunks

| # | SHA | What | `src/` hunks under `git show --stat` |
|---|---|---|---|
| 1 | `f964926` | the fifth evidence file — the words the shell deletes | **0** |
| 2 | `aa24f9d` | `DELETION_CLASSES` — a second axis beside the seven assembly classes | **0** |
| 3 | `1e6a2c8` | the execution record, the registrations, and the handoff gate | **0** |

`git diff --numstat fccb5de HEAD -- src/` is empty. `git diff --numstat fccb5de
HEAD -- Cargo.toml Cargo.lock` is empty (`T-19-SC` holds). Four files changed in
total, 2850 insertions, and both `.planning/` diffs are pure appends with zero
deleted lines.

## The gate, with the arithmetic stated and checked

Command: `rtk proxy cargo test --no-fail-fast`. Plain `cargo test` fail-fasts at
`driver_reattach` and `envelope_*` sorts after `driver_*`, so an unqualified run
never executes a single envelope suite; `rtk proxy` is required because the RTK
hook strips exactly the `warning:` and `test result:` lines any count is read
from (D-34).

| | passed | failed | ignored | `passed + failed` |
|---|---|---|---|---|
| Before (`19-17-SUMMARY.md`, re-measured by audit 5) | 1584 | 0 | 13 | **1584** |
| After (this plan) | 1600 | 5 | 13 | **1605** |

**The identity holds exactly.** A red test RAN, so red→green contributes
nothing and the increase comes only from NEW `#[test]` fns. Counted from
`git show`: `tests/envelope_argv_deletion.rs` contributes **17**,
`tests/envelope_wrapper_class.rs` contributes **4**
(`the_corpus_can_draw_every_one_of_the_five_deletion_classes`,
`every_alphabet_this_round_widens_can_draw_a_word_the_shell_deletes`,
`the_generated_corpus_really_produces_each_deletion_class_in_quantity`,
`a_word_the_shell_deletes_between_the_program_and_its_decision_words_is_not_a_decision_word`).

```
1584 + 17 + 4 = 1605   =   observed passed + failed
```

**New `#[test]` fn count: 21. Post-state `passed + failed`: 1605.** Both are
`19-19`'s handoff numbers.

## THE RED SET — `19-19`'s handoff contract

Five names, and every one of them is a name this plan created. **This is the
complete list.**

```
after_19_19_a_redirection_between_the_program_and_its_decision_words_is_refused
after_19_19_a_line_continuation_inside_or_beside_a_decision_word_is_refused
after_19_19_the_planning_cells_are_refused_because_they_are_the_same_class
after_19_19_the_forge_rows_are_counted_rather_than_bypassed_uncounted
a_word_the_shell_deletes_between_the_program_and_its_decision_words_is_not_a_decision_word
```

The first four are in `tests/envelope_argv_deletion.rs`; the fifth is
`tests/envelope_wrapper_class.rs` section 14c.

**The two flaky `driver_reattach` tests both PASSED in this run** — `3 passed;
0 failed`. That is not a change this plan made and it is not a fix; they remain
documented, flaky and out of scope.

## Per-binary counts — every envelope suite

| Binary | passed | failed |
|---|---|---|
| `envelope_advisory` | 10 | 0 |
| **`envelope_argv_deletion`** (new) | **13** | **4** |
| `envelope_command_position` | 18 | 0 |
| `envelope_credential` | 6 | 0 |
| `envelope_expansion_slots` | 32 | 0 |
| `envelope_hook_refusals` | 7 | 0 |
| `envelope_literal_decision` | 43 | 0 |
| `envelope_pr_cap` | 11 | 0 |
| `envelope_tracer` | 6 | 0 |
| `envelope_wiring` | 14 | 0 |
| `envelope_wrapper_bypass` | 13 | 0 |
| **`envelope_wrapper_class`** | **28** | **1** |

`envelope_literal_decision` (43), `envelope_expansion_slots` (32) and
`envelope_command_position` (18) are FULLY GREEN and byte-identical — rounds 4's
and 5's evidence stays attributable to the round that produced it.

## Every measured row

All measured against the BUILT BINARY at `fccb5de`, one fresh
`GSD_MM_ENVELOPE_ROOT` per row via `mktemp -d`, the whole root WALKED afterwards
so an absent ledger line is OBSERVED rather than inferred. **Every row was
driven before it was written as an assertion.**

### `T-19-97` — the sixteen audit-5 rows

| Command | exit | walk | bash-shim argv |
|---|---|---|---|
| `git >/dev/null push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git 2>/dev/null push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git 1>/dev/null push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git >>/tmp/x push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git > /tmp/o push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git <<<x push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git push>/dev/null --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git >/dev/null stash` | 0 | empty | `[stash]` |
| `git >/dev/null update-ref -d refs/heads/main` | 0 | empty | `[update-ref] [-d] [refs/heads/main]` |
| `git >/dev/null config core.hooksPath /tmp/x` | 0 | empty | `[config] [core.hooksPath] [/tmp/x]` |
| `gh >/dev/null pr create --title x` | 0 | **empty** | `ARGV[gh]: [pr] [create] [--title] [x]` |
| `gh >/dev/null api repos/o/r/pulls -f title=x` | 0 | **empty** | `ARGV[gh]: [api] [repos/o/r/pulls] [-f] [title=x]` |
| `glab >/dev/null mr create --title x` | 0 | **empty** | `ARGV[glab]: [mr] [create] [--title] [x]` |
| `bash -lc "git >/dev/null push --force origin main"` | 0 | empty | (nested payload) |
| `echo hi && git >/dev/null push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git >/dev/null -c core.hooksPath=/dev/null push --force origin main` | 0 | empty | `[-c] [core.hooksPath=/dev/null] [push] [--force] [origin] [main]` |

Three of these have **no second carrier at all** — `git stash`,
`git update-ref`, and the SAFE-06 cap; `git config core.hooksPath` is the loss
of the second carrier itself.

**The three-leg argument, each leg re-measured.** Layer 1: the line begins
`git >`, which does not match the `Bash(git push:*)` prefix rule. Layer 2:
defeated on the same line (row 16, exit 0, empty walk). Layer 3: disarmed —
`GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath
GIT_CONFIG_VALUE_0=/ENV_WINS git >/tmp/l3out -c core.hooksPath=/CLI_WINS config
--get core.hooksPath` prints **`/CLI_WINS`**, with the paired control (same line
without `-c`) printing **`/ENV_WINS`** (D-09).

### `T-19-98` — the eight audit-5 rows

| Command | exit | walk | bash-shim argv |
|---|---|---|---|
| `git pu\<NL>sh --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git \<NL>push --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `git sta\<NL>sh` | 0 | empty | `[stash]` |
| `gh p\<NL>r create --title x` | 0 | **empty** | `ARGV[gh]: [pr] [create] [--title] [x]` |
| `git config core.hooks\<NL>Path /tmp/x` | 0 | empty | `[config] [core.hooksPath] [/tmp/x]` |
| `git "pu\<NL>sh" --force origin main` | 0 | empty | `[push] [--force] [origin] [main]` |
| `bash -lc "git pu\<NL>sh --force origin main"` | 0 | empty | (nested payload) |
| `git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force origin main` | 0 | empty | `[-c] [core.hooksPath=/dev/null] [push] [--force] [origin] [main]` |

**No audit-5 row failed to reproduce.** All 24 came up at exactly the verdict
audit 5 recorded, so nothing here asserts a number over a disagreeing
measurement.

### The `od -c` bytes for the backslash-newline rows

Every `T-19-98` row was written to a FILE, its bytes checked, and bash driven
over the file — the step audit 5 discarded three candidates on.

```
git pu\<NL>sh --force origin main
  0000000   g   i   t       p   u   \  \n   s   h       -   -   f   o   r
  0000020   c   e       o   r   i   g   i   n       m   a   i   n  \n

git \<NL>push --force origin main
  0000000   g   i   t       \  \n   p   u   s   h       -   -   f   o   r
  0000020   c   e       o   r   i   g   i   n       m   a   i   n  \n

git sta\<NL>sh
  0000000   g   i   t       s   t   a   \  \n   s   h  \n

gh p\<NL>r create --title x
  0000000   g   h       p   \  \n   r       c   r   e   a   t   e       -
  0000020   -   t   i   t   l   e       x  \n

git config core.hooks\<NL>Path /tmp/x
  0000000   g   i   t       c   o   n   f   i   g       c   o   r   e   .
  0000020   h   o   o   k   s   \  \n   P   a   t   h       /   t   m   p
  0000040   /   x  \n

git "pu\<NL>sh" --force origin main
  0000000   g   i   t       "   p   u   \  \n   s   h   "       -   -   f
  0000020   o   r   c   e       o   r   i   g   i   n       m   a   i   n
  0000040  \n

git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force origin main
  0000000   g   i   t       -   c       c   o   r   e   .   h   o   o   k
  0000020   s   \  \n   P   a   t   h   =   /   d   e   v   /   n   u   l
  0000040   l       p   u   \  \n   s   h       -   -   f   o   r   c   e
  0000060       o   r   i   g   i   n       m   a   i   n  \n

git push \<NL> origin refs/heads/gsd-auto/alpha/w
  0000000   g   i   t       p   u   s   h       \  \n       o   r   i   g
  0000020   i   n       r   e   f   s   /   h   e   a   d   s   /   g   s
  0000040   d   -   a   u   t   o   /   a   l   p   h   a   /   w  \n
```

**A shim-harness finding worth recording.** The first shim draft printed argv to
stdout and produced NOTHING for eleven rows — precisely the rows whose point is
`>/dev/null`. The shims now write argv to a side file (`$ARGV_LOG`), so a row
cannot swallow the very evidence it is driven for.

### The seven cells found while PLANNING round 6

Same provenance caveat `19-14` and `19-16` established: **found while planning,
not by an audit.** All seven at exit 0 with empty walks, all confirmed under the
shims, and all folded into `T-19-97`'s class in `deferred-items.md` rather than
registered as new threat IDs.

| Cell | Line | exit | argv | Why the audit's rows do not reach it |
|---|---|---|---|---|
| `&>` | `git &>/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | `&` is in `SEPARATORS`, so the GUARD splits one simple command into two; bash's `&>` is ONE operator |
| `{v}>` | `git {v}>/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | bash 4.1 fd allocation, carried into the verb slot by round 5's own literal-brace branch. **Post-fix verdict left for `19-19`** |
| `>\|` | `git >\|/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | a two-character operator |
| `<>` | `git <>/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | a two-character operator, first char `<` and second `>` |
| `<<` | `git <<EOF push --force origin main` | 0 | `[push] [--force] [origin] [main]` | bash runs it even as a single line (it warns, then runs) |
| `<file` | `touch input.txt && git <input.txt push --force origin main` | 0 | `[push] [--force] [origin] [main]` | **PRECONDITION row** |
| `x2>` | `git x2>/tmp/o push --force origin main` | 0 | **`[x2] [push] [--force] [origin] [main]`** | **the OVER-DELETION control** — pinned PERMITTED |

Two further operators were measured because they are the ones `19-19`'s
production uniquely adds: `git &>>/tmp/o push --force origin main` and
`git <<-EOF push --force origin main` at exit 0 with `[push] [--force] [origin]
[main]`, and their permitted twins `git &>>/tmp/o status` and `git <<-EOF
status` at exit 0 with `[status]`. Both are entries in
`DISPLACING_REDIRECTIONS`, so the corpus can fail on them.

### The position controls — where the hole is NOT

```
exit=2  force_push_blocked   >/dev/null git push --force origin main       (already refused)
exit=2  force_push_blocked   2>/dev/null git push --force origin main      (already refused)
exit=2  force_push_blocked   env >/dev/null git push --force origin main   (already refused)
exit=2  force_push_blocked   git push --force origin main >/dev/null       (already refused)
exit=0  + ONE LEDGER LINE    >out gh pr create --title x                   (already COUNTED)
                             walked listing: alpha/pr-ledger.ndjson (103 bytes)
```

`resolve_program` walks past a leading redirection as it would any wrapper
operand and finds `git`. **The hole is a redirection standing BETWEEN the
governed program and its decision words**, which is exactly where
`DISPLACING_REDIRECTIONS` splices.

### The PERMITTED half, and the measured cost of the rejected design option

All at exit 0 today and required to stay so:

```
exit=0  git >/dev/null status                    -> ARGV[git]: [status]
exit=0  git >/dev/null log --oneline             -> ARGV[git]: [log] [--oneline]
exit=0  git log > out
exit=0  git status > /tmp/s.txt
exit=0  git diff > /tmp/d.patch
exit=0  git commit -m "x" >> build.log
exit=0  gh pr list 2>/dev/null
exit=0  git fetch origin 2>&1 | tee log
exit=0  git commit -m ">"                        (a QUOTED `>` is an ordinary character)
exit=0  git log --grep='>'
exit=0  git commit -m "a > b"
exit=0  rg ">" src/
exit=0  git push origin refs/heads/gsd-auto/alpha/w --push-option="a>b"
exit=0  + ONE LEDGER LINE   gh pr create --title x > /tmp/o
```

**The design question, answered.** The REJECTED option is: refuse any governed
simple command containing a token the shell deletes. It would refuse every row
above, and it would turn `gh pr create --title x > /tmp/o` — **COUNTED today,
one ledger line** — into a refusal, trading a correct COUNT for a false positive
on ordinary forge syntax. That is the exact trade `T-19-93`'s bar forbids.
`19-19` models the deletion instead, and the record states the choice **before**
the rule exists so it is auditable rather than retrofitted.

### The OVER-DELETION control

```
exit=0  git x2>/tmp/o push --force origin main
        -> ARGV[git]: [x2] [push] [--force] [origin] [main]
```

An IO_NUMBER is a digits-only run since the START of the word, so bash ends the
word `x2` and begins the redirection `>/tmp/o`. `x2` **IS argv**, and real git
answers `git: 'x2' is not a git command`. Permitted today and it must STAY
permitted: after the fix the guard's argv equals git's argv. **This is this
axis's `ls {git,svn}-repo`** — the row that tells a deletion MODEL apart from a
rule that deletes any word part before a `>` and displaces every decision word
LEFT.

### The two pre-existing FALSE REFUSALS — the round REMOVES over-refusal

Measured against a repository whose current branch is inside
`refs/heads/gsd-auto/alpha/` with a local bare upstream:

```
exit=2 push_outside_namespace   git push origin refs/heads/gsd-auto/alpha/w > log.txt
  "the refspec `>` resolves to `refs/heads/>`, which is outside `refs/heads/gsd-auto/alpha/`"
exit=0                          git push origin refs/heads/gsd-auto/alpha/w      (twin)

exit=2 push_outside_namespace   git push \<NL> origin refs/heads/gsd-auto/alpha/w
  "the refspec `origin` resolves to `refs/heads/origin`, which is outside
   `refs/heads/gsd-auto/alpha/`"
exit=0                          git push origin refs/heads/gsd-auto/alpha/w      (twin)
```

Both derivations are taken from the MEASURED message, and they are **different
mechanisms**. The redirection word is read as an EXTRA refspec. The
`\`+newline is flushed by the whitespace after it and becomes its OWN word in
the REMOTE slot, displacing every operand one slot right — `T-19-97`'s
displacement arriving through `T-19-98`'s mechanism. Under the shims both lines
print `ARGV[git]: [push] [origin] [refs/heads/gsd-auto/alpha/w]`.

Pinned pre-fix in the two `#[test]` fns this plan names, which are the ONLY
assertions in `tests/envelope_argv_deletion.rs` that `19-19` is permitted to
REPLACE rather than only add to:

* `the_redirected_in_namespace_push_is_falsely_refused_today_and_19_19_must_move_it`
* `the_continued_in_namespace_push_is_falsely_refused_today_and_19_19_must_move_it`

**The round removes two measured over-refusals rather than adding any.**

## The rows LABELLED as PRECONDITION rows

* `touch input.txt && git <input.txt push --force origin main` — without the
  file bash fails the redirection and runs NOTHING. Asserted in its `touch …&&`
  spelling, the same standard audit 4 applied to `touch push && git pus? …`.
* `git >$F push --force origin main` and `git >*.log push --force origin main` —
  bash answers `ambiguous redirect` when `F` is unbound and when the glob matches
  more than one file. Both are also undeliverable-verdict rows and are recorded
  rather than asserted.

## Audit 5's three DISCARDED rows were NOT re-added

Single-quoted `\`+newline, `\`+CR and `\`+TAB stay discarded and are rows in no
file. **Disclosure:** the single-quoted spelling was re-run ONCE as a sanity
check of this round's shim harness. It printed `ARGV[git]: [pu\` / `sh]
[--force] [origin] [main]` — one mangled word and no force push — confirming
audit 5's discard. It was not added as a row.

## The rows whose POST-fix verdict is left for `19-19`

`tests/envelope_argv_deletion.rs` section 9 drives each and PRINTS its verdict
with **no assertion about it**. The test body contains no `assert` on any of the
three.

| Row | measured at `fccb5de` | why `19-19` must derive it |
|---|---|---|
| `git >$F push --force origin main` | exit 2 `envelope_assertion_failed` — *``>$F`` is the git verb for this command* | round 4's bit fires on the `$` in what the guard reads as the VERB. After deletion the surviving argv is `push --force origin main`, so it stays refused with a DIFFERENT identifier |
| `git >*.log push --force origin main` | exit 2 `envelope_assertion_failed` — same shape | the same, through the glob |
| `git {v}>/tmp/o push --force origin main` | exit 0, empty walk | `19-19` deliberately does not model a `{name}` fd prefix and marks it unresolvable — a different identifier reached by a different clause from its six siblings. Its permitted twin `git {v}>/tmp/o status` is exit 0 and recorded the same way |

For the same reason `{v}>/tmp/o` is kept OUT of `DISPLACING_REDIRECTIONS`: every
entry there is asserted verdict-PRESERVING by the property's permitted arm, and
this one would be STRICTER than its base and would turn the property permanently
red in a file `19-19` may not edit.

## The FOUR FORGE rows assert a COUNT, not a refusal

```
gh   >/dev/null pr create --title x            -> exit 0, EMPTY WALK   (RED)
gh   >/dev/null api repos/o/r/pulls -f title=x -> exit 0, EMPTY WALK   (RED)
glab >/dev/null mr create --title x            -> exit 0, EMPTY WALK   (RED)
gh   p\<NL>r create --title x                  -> exit 0, EMPTY WALK   (RED)
```

Each UNWRAPPED form is measured at exit 0 with exactly ONE ledger file —
`gh pr create --title x` → `alpha/pr-ledger.ndjson (103 bytes)`,
`gh api repos/o/r/pulls -f title=x` → 112 bytes,
`glab mr create --title x` → 105 bytes. After deletion the surviving argv IS the
counted one, so `pr_command_label` matches and the creation is COUNTED. The
correct post-fix verdict is **exit 0 with exactly one ledger line in a walked
fresh root**.

**They are RED today because the walk is EMPTY**, which is precisely the defect:
the SAFE-06 cap is bypassed **UNCOUNTED rather than exceeded**, and it has no
second carrier (`T-19-35`). Asserting exit 2 would have pinned four assertions
at a verdict `19-19` cannot produce.

## The SECOND AXIS

`tests/envelope_wrapper_class.rs` section 14. The diff is **1010 insertions and
ZERO deletions**: `UNREADABLE_CLASSES`, its seven predicates, `brace_pairs`, the
degenerate-proofing block, `MIN_UNREADABLE_GENERATED_CASES`,
`MIN_GENERATED_CASES_PER_UNREADABLE_CLASS` and `MIN_UNREADABLE_FORGE_SLOT_CASES`
are byte-identical, and no existing floor, alphabet entry, property or assertion
was lowered, deleted or narrowed.

**`DELETION_CLASSES` — five classes, each degenerate-proof and quoting-aware:**

1. separate-word redirection (an operator standing as its own word or leading
   one, with a following word)
2. **ATTACHED** redirection (a non-IO_NUMBER literal run before the operator
   inside the same word) — class 1 cannot satisfy it
3. a multi-character or fd-carrying operator (`>>`, `2>`, `1>`, `<<<`, `<<`,
   `<<-`, `>|`, `<>`, `>&`, `<&`, `&>`, `&>>`) — a bare `>` cannot satisfy it
4. a continuation INSIDE a word — class 5 cannot satisfy it
5. a continuation at a word BOUNDARY — class 4 cannot satisfy it

`git log --grep='>'` and `git commit -m "a > b"` are asserted to satisfy
**NONE**.

**`DISPLACING_REDIRECTIONS`** (11 entries) splices BETWEEN the governed program
and its decision words. `&>>/tmp/o` and `<<-EOF` are required entries — the two
spellings `19-19`'s production uniquely adds. `{v}>/tmp/o` is asserted ABSENT
with the reason in the doc. **`CONTINUATION_SPLICES`** carries the two
backslash-newline splice points as splice POINTS rather than strings, because
what distinguishes `T-19-98`'s two classes is where the pair sits.

**The generative property** runs the positive walk control, then the base
floors, then the **PERMITTED arm** (132 cases asserting the verdict is
UNCHANGED — green today, and run first so today's failing output is itself
evidence the permitted half passed), then the **REFUSED arm** (198 cases
asserting refusal AND an empty walk — RED today). Measured while writing: **124
of the 198 refused-arm cases are at exit 0 today**; the other 74 are already
refused for a position the splice did not reach. All 132 permitted-arm cases are
at exit 0 with no anomalies.

**Floors as EXACT equalities with the arithmetic stated**: 11 entries, 9 of them
attachable, 2 continuation splices → 22 cases per slot; 9 refused slots + 6
permitted slots = 15 → **330 cases**, per class **165 / 135 / 240 / 15 / 15**.

## Deviations from Plan

### An execution-time FINDING, reported rather than absorbed

**[Rule 1 - Bug] The first draft of `draws_an_attached_redirection` over-counted
class 2 by 90 cases.**

* **Found during:** Task 2, by the exact per-class equality itself.
* **Issue:** the predicate asked "does some operator have a non-empty run before
  it", which reads the second `>` of `>>/tmp/x` as an operator attached to a
  literal run `>`. Six of the eleven entries — `>>/tmp/x`, `<<<x`, `<>/tmp/o`,
  `&>/tmp/o`, `&>>/tmp/o` and `<<-EOF` — are SEPARATE-WORD redirections and every
  one of them also satisfied the ATTACHED class. Measured 225 against the derived
  135. **That collapses the class-1/class-2 split the whole axis turns on**: a
  corpus of separate-word redirections would have satisfied the attached floor
  and could never have failed on `git push>/dev/null --force …`.
* **Fix:** encode bash's actual rule — take the LONGEST operator match at the
  start of the redirection; everything after it is the TARGET, never a literal
  run. Class 2 is now "the FIRST operator occurrence in the word sits past the
  IO_NUMBER".
* **Why it was caught:** the floor is an `assert_eq!` against a stated
  derivation. **A `>=` floor would not have caught it** — 225 is comfortably
  above 135.
* **Pinned:** seven new degenerate-proofing assertions, one per multi-character
  spelling, asserting each is NOT attached.
* **Files:** `tests/envelope_wrapper_class.rs`. **Commit:** `aa24f9d`.

### Deferred Issues (out of scope, logged not fixed)

**`cargo clippy --tests -- -D warnings` already fails at the base commit.**
Verified by removing this plan's new test file and re-running at `fccb5de`: exit
101, four lint errors, all in `src/` files this plan does not touch — three
`clippy::bool_assert_comparison` at `src/browser.rs:155-157` and one
`clippy::cmp_owned` at `src/project_creator.rs:146`. **This plan's new code adds
ZERO clippy findings** (`cargo clippy --tests` names neither new file). Fixing
these requires a `src/` hunk, which this plan is prohibited from making.
Recorded in `deferred-items.md`.

### Nothing else deviated

No row failed to reproduce. No expectation was adjusted to match a measurement
that disagreed with the audit. No `assert` was written on any of the three
undeliverable rows.

## `T-19-17r` — the bookkeeping gap, OUTSTANDING

`19-17-SUMMARY.md` calls `T-19-17r` "the new over-refusal cost, **accepted** and
pinned from both sides". Audit 5 confirmed the measurement and both pins —
`rg "git status" {src,tests}` → exit 2 and `rg "git status" src/` → exit 0, at
`tests/envelope_literal_decision.rs:1355-1379` with the producing clause named
— but there is **no `AR-19-13` row in the Accepted Risks Log and no register
row**. It is described as accepted while documented nowhere an audit reads.

**This plan did NOT accept it and added NO `AR-19-13` row.** The Accepted Risks
Log still ends at `AR-19-12`. Every occurrence of the string `AR-19-13` this plan
wrote is a statement that the row does not exist. Accepting a risk is a human
decision and audit 5 explicitly declined to make it on the developer's behalf;
the next round either adds the log row or drops the word from the summary.

## Audit 5's disclosed corpus limit, forwarded to `19-19`

`sh {-c,"git push --force …"}` is refused because a quote inside an alternative
is unenumerable. That is CORRECT and fail-closed, but the corpus cannot
distinguish that refusal from an enumerated one. **Forwarded to `19-19` rather
than addressed here because the only place the distinction is observable is a
unit assertion over the private whole-word product scan in `policy.rs`'s own
test module, and this plan may not touch `src/`.**

## What remains uncovered

1. **`T-19-86`** (high, OPEN) — a governed program's own operand naming a
   governed command. Open by **explicit user scoping decision**. All four rows
   still exit 0;
   `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted` is
   green and UNMODIFIED. Because it remains open at `high`, this plan does not
   clear `/gsd-secure-phase 19`.
2. **`T-19-91`** (high, OPEN) — a git classifier's own decision operand,
   assembled by expansion. **Three arms**, unweakened: `git reflog $S`,
   `git reflog show $S` and `git symbolic-ref $S` at exit 0 **with no second
   carrier**, and the bare `git push $REF` cwd-dependent and permitted
   in-namespace — a THIRD arm and not "already fails closed". No remedy added, no
   denylist extended.
3. **`T-19-96`** (medium, open) — a glob in a PUSH FLAG, one slot outside the
   decision region. Left exactly as `19-16` pinned it.
4. **`T-19-74`** — accepted (AR-19-10); core rows frozen and re-measured
   permitted.
5. **`T-19-84`, `T-19-85`** — open, unaccepted, untouched.
6. **`T-19-61` … `T-19-73`** — open, unaccepted, untouched by explicit user
   decision. `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not
   opened; the honesty statement, the second-carrier table and the park-coverage
   surface were not touched.
7. **`T-19-97`, `T-19-98`, `T-19-99`** — the subject of this round, all OPEN. The
   corpus exists and is RED; `19-19` writes the rule.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` are both sub-classes of it and both remain OPEN at `high`, so neither
this plan nor `19-19` clears `/gsd-secure-phase 19`.

## Self-Check: PASSED

* `tests/envelope_argv_deletion.rs` — FOUND
* `tests/envelope_wrapper_class.rs` — FOUND, +1010 / −0
* `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md` — FOUND, +373 / −0
* `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` — FOUND, +165 / −0
* commit `f964926` — FOUND, 0 `src/` hunks
* commit `aa24f9d` — FOUND, 0 `src/` hunks
* commit `1e6a2c8` — FOUND, 0 `src/` hunks
* `git diff --numstat fccb5de HEAD -- src/ Cargo.toml Cargo.lock` — empty
