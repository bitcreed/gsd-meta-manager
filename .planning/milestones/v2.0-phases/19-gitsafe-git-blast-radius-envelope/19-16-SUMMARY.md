---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 16
subsystem: envelope
tags: [security, test-corpus, gap-closure, red-state]
status: complete
requires:
  - "19-15's Rule B and its severed-prefix alphabet"
  - "audit 4's T-19-92 … T-19-95 findings in 19-SECURITY.md"
provides:
  - "tests/envelope_literal_decision.rs — round 5's evidence file, 41 tests, 21 RED"
  - "seven unreadable classes with degenerate-proof predicates and three kinds of floor"
  - "the exact handoff number 19-17 gates against: passed + failed = 1580"
affects:
  - "19-17, which confirms the named RED tests still red before it writes a line"
tech-stack:
  added: []
  patterns:
    - "measure against the built binary BEFORE writing an assertion"
    - "confirm shell semantics with argv-printing shims; label coverage rows as coverage"
    - "assert a LEDGER LINE where the bar is count rather than refuse"
    - "pin the mechanism, not only the verdict, when a later plan changes the tokenizer"
key-files:
  created:
    - tests/envelope_literal_decision.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "A comma list of N alternatives produces N words, so only the RANGE spellings of the concatenated splice class assemble a real force push; the comma spellings are kept and labelled COVERAGE rather than claimed as reproduced harms"
  - "git push {--force,origin} main is cwd-dependent — exit 0 in namespace, exit 2 outside it under an unrelated arm — so it is pinned in the in-namespace configuration with the repository passed explicitly"
  - "The quoted-run spellings stay out of REFUSED_BASES because the outer ShellLayers re-quote the whole payload and would emit illegal shell"
  - "SEVERED_PREFIXES was not widened; a separate SEVERED_BRACE_PREFIXES alphabet was added instead, because the existing property asserts every generated case carries a live $ or backtick and that floor must not be lowered"
  - "No glob entry was added to EXPANSION_WRAPPERS: 19-17 does not move resolve_program steps 3 and 5, so a glob wrapper prefix would be pinned at a refusal the rule cannot produce"
metrics:
  duration: "~1 session"
  completed: 2026-08-29
actuals:
  tokens: 78000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 16: The Round-5 Corpus, RED — Summary

The corpus and the reproducers for round 5, measured against the built binary and
committed RED before a single line of the rule exists; `19-17` writes the rule.

## Read this first: what is NOT closed

**`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-17`, or by the two
together.**

* **`T-19-86`** — OPEN at `high`, by explicit user scoping decision. Untouched and
  unremediated. `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`
  is green and unmodified, all four rows still at exit 0.
* **`T-19-91`** — OPEN at `high`. This plan **measures** the in-namespace
  `git push $REF` and records it; the code-side record correction is `19-17`'s.
  No decision-operand rule was added for `reflog`, `symbolic-ref` or `push`.
* **`T-19-92`, `T-19-93`, `T-19-94`, `T-19-95`** — all still OPEN. This plan
  closes nothing. `T-19-95` is closed only once `19-17`'s rule is certified by the
  corpus written here, because a corpus is evidence about a control and there is
  no control yet.
* **`T-19-96`** — registered, measured, pinned, **not fixed**.
* **`T-19-74`** core rows frozen and re-measured green; **`T-19-61` … `T-19-73`,
  `T-19-84`, `T-19-85`** open, unaccepted, untouched.

Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed. `T-19-86` and
`T-19-91` are both sub-classes of it and both remain open at `high`.

## The handoff `19-17` gates against

| | value |
|---|---|
| `passed + failed` BEFORE (`cargo test --no-fail-fast`) | **1533** (1533 passed / 0 failed / 13 ignored) |
| `passed + failed` AFTER | **1580** (1552 passed / 28 failed / 13 ignored) |
| new `#[test]` fns added | **47** (41 + 6) |
| arithmetic | 1533 + 47 = 1580, exactly. A red test *ran*, so red→green leaves the total unchanged; the increase is the new fns and nothing else |

Mode-independent: the `driver_reattach` pair always RUNS, so `passed + failed` is
1580 in both flake modes. The baseline run had that pair passing; the post-state
run had it failing.

### The complete list of failing test names

**26 are this plan's RED set.** `19-17` confirms every one still red before it
writes a line.

`tests/envelope_literal_decision.rs` — 21:

```
after_19_17_a_brace_expansion_at_position_zero_is_refused
after_19_17_a_brace_expansion_in_a_forge_command_is_refused_and_writes_no_ledger_line
after_19_17_a_brace_expansion_in_an_operand_of_a_governed_command_is_a_pinned_cost
after_19_17_a_brace_expansion_spliced_into_a_governed_command_is_refused
after_19_17_a_c_key_half_carrying_a_glob_is_refused_even_quoted_and_that_is_disclosed
after_19_17_a_concatenated_splice_in_a_flag_slot_is_refused_and_this_one_is_genuine
after_19_17_a_glob_in_a_decision_word_is_refused
after_19_17_a_glob_in_a_gh_api_endpoint_is_refused_and_writes_no_ledger_line
after_19_17_a_glob_in_a_git_config_key_operand_is_refused
after_19_17_a_second_gh_placeholder_creation_in_one_run_is_refused_under_the_cap
after_19_17_a_splice_concatenated_with_literal_runs_is_refused_comma_spellings
after_19_17_a_splice_concatenated_with_literal_runs_is_refused_range_spellings
after_19_17_a_splice_in_a_governed_verb_slot_with_a_harmless_surplus_is_refused
after_19_17_a_splice_into_a_push_flag_slot_is_refused_in_the_in_namespace_configuration
after_19_17_a_tilde_in_a_decision_word_is_refused_and_this_row_is_coverage
after_19_17_an_unquoted_glob_in_a_config_key_operand_is_a_pinned_cost
after_19_17_clause_2b_over_refuses_an_ungoverned_command_and_that_is_a_pinned_cost
after_19_17_the_brace_spellings_with_no_second_carrier_are_refused
after_19_17_the_forge_twins_of_the_produce_class_are_refused_and_write_no_ledger_line
after_19_17_the_gh_placeholder_endpoint_is_permitted_and_counted
after_19_17_the_glob_precondition_satisfied_inside_one_tool_call_is_refused
```

`tests/envelope_wrapper_class.rs` — 5:

```
a_forge_decision_slot_carrying_a_splice_or_a_glob_is_refused_in_every_generated_slot
a_gh_placeholder_endpoint_is_permitted_and_counted_in_every_generated_placement
a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position
a_wrapper_prefix_carrying_an_expansion_is_refused_in_every_generated_position
the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command
```

**2 are the documented-flaky pair, out of scope and untouched:**

```
a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step
a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired
```

**No third kind of name appears.**

### Per-binary counts

| binary | result |
|---|---|
| `--test envelope_literal_decision` | FAILED. 20 passed; 21 failed; 0 ignored |
| `--test envelope_wrapper_class` | FAILED. 20 passed; 5 failed; 0 ignored |
| `--test envelope_expansion_slots` | ok. 32 passed; 0 failed |
| `--test envelope_command_position` | ok. 18 passed; 0 failed |
| `--lib envelope` | ok. 184 passed; 0 failed |
| `--lib envelope::policy` | ok. 61 passed; 0 failed |

Round-4's evidence is untouched and fully green. `cargo build` and
`cargo clippy -- -D warnings` exit 0; `cargo clippy --all-targets` reports the
same **four** pre-existing warnings (three in `src/browser.rs:155-157`, one in
`src/project_creator.rs:146`) and no fifth.

## Verbatim RED output

Task 1, `rtk proxy cargo test --test envelope_literal_decision`:

```
test result: FAILED. 20 passed; 21 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

with, for example:

```
thread 'after_19_17_a_brace_expansion_spliced_into_a_governed_command_is_refused' panicked at
tests/envelope_literal_decision.rs:267:5:
assertion `left == right` failed: `git {-c,core.hooksPath=/dev/null,push,--force,origin,main}`
must be REFUSED. It was measured at exit 0 against the built binary at this file's base commit.
stdout:  stderr:
  left: 0
 right: 2

thread 'after_19_17_the_gh_placeholder_endpoint_is_permitted_and_counted' panicked at
tests/envelope_literal_decision.rs:369:5:
assertion `left == right` failed: `gh api repos/{owner}/{repo}/pulls -f title=x` must leave
EXACTLY ONE pull-request ledger line. An uncounted creation is the SAFE-06 cap bypassed rather
than exceeded, and the cap has no second carrier (`T-19-35`). Walked listing:
(the envelope root is empty)
  left: 0
 right: 1

thread 'after_19_17_a_splice_into_a_push_flag_slot_is_refused_in_the_in_namespace_configuration'
panicked at tests/envelope_literal_decision.rs:842:5:
assertion `left == right` failed: `git push {--force,origin} main` must be REFUSED in the
IN-NAMESPACE configuration. It was measured at exit 0 there against the built binary at this
file's base commit, while the same line measured exit 2 outside the namespace for an unrelated
reason — so a control that only ever ran out of namespace would be green while the class stayed
wide open.
  left: 0
 right: 2
```

Task 2, `rtk proxy cargo test --test envelope_wrapper_class`:

```
test result: FAILED. 20 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

thread 'the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command' panicked at
tests/envelope_wrapper_class.rs:601:9:
assertion `left == right` failed: the UNWRAPPED base `git {push,--force} origin main` must itself
be refused. If this row is red, every invariance assertion below compares a permit against a
permit and proves nothing — the harness cannot observe a denial at all.
Got reason id: permit(the guard answered nothing)

thread 'a_wrapper_prefix_carrying_an_expansion_is_refused_in_every_generated_position' panicked at
tests/envelope_wrapper_class.rs:2055:9:
assertion `left == right` failed: the UNWRAPPED base `git {push,--force} origin main` must itself
be refused, or every assertion below is comparing a refusal against nothing.

thread 'a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position'
panicked at tests/envelope_wrapper_class.rs:1556:9:
assertion `left == right` failed: the UNWRAPPED base `git {push,--force} origin main` must itself
be refused, or every decoy assertion below is comparing a refusal against nothing.
```

The three shared-alphabet properties fail at **floor 2**, which measures every
base refused UNWRAPPED before the wrapping loop runs. That is precisely the
non-vacuity the widening was for: those 5180 generated cases previously certified
a fix against a class the corpus could not draw.

## Measured verdicts

### Audit 4's fourteen rows — all reproduced, none reported as a non-reproducer

Driven at `e842fa3` against `./target/debug/gsd-meta-manager`, one fresh
`GSD_MM_ENVELOPE_ROOT` per row, envelope directory walked afterwards.

```
exit=0  git {-c,core.hooksPath=/dev/null,push,--force,origin,main}   empty walk   T-19-92
exit=0  git {push,--force} origin main                                            T-19-92
exit=0  git {update-ref,-d,refs/heads/main}                                       T-19-92
exit=0  git {config,core.hooksPath,/tmp/x}                                        T-19-92
exit=0  gh {pr,create} --title x                                     empty walk   T-19-92
exit=0  gh {api,repos/o/r/pulls} -f title=x                          empty walk   T-19-92
exit=0  glab {mr,create} --title x                                   empty walk   T-19-92
exit=0  bash -lc "git {push,--force} origin main"                                 T-19-92
exit=0  gh api repos/{owner}/{repo}/pulls -f title=x                 empty walk   T-19-93
exit=0  git pus? --force origin main                                              T-19-94
exit=0  git ?ush --force origin main                                              T-19-94
exit=0  git stas?                                                                 T-19-94
exit=0  gh p? create --title x                                       empty walk   T-19-94
exit=0  touch push && git pus? --force origin main                                T-19-94
```

Bash shim confirmations (`git`/`gh`/`glab` first on `PATH`, printing argv):

```
git {-c,core.hooksPath=/dev/null,push,--force,origin,main}
  ARGV[git]: [-c] [core.hooksPath=/dev/null] [push] [--force] [origin] [main]
git {push,--force} origin main            ARGV[git]: [push] [--force] [origin] [main]
git {update-ref,-d,refs/heads/main}       ARGV[git]: [update-ref] [-d] [refs/heads/main]
git {config,core.hooksPath,/tmp/x}        ARGV[git]: [config] [core.hooksPath] [/tmp/x]
gh {pr,create} --title x                  ARGV[gh]:  [pr] [create] [--title] [x]
gh {api,repos/o/r/pulls} -f title=x       ARGV[gh]:  [api] [repos/o/r/pulls] [-f] [title=x]
glab {mr,create} --title x                ARGV[glab]:[mr] [create] [--title] [x]
bash -lc "git {push,--force} origin main" ARGV[git]: [push] [--force] [origin] [main]
printf "[%s]" repos/{owner}/{repo}/pulls  [repos/{owner}/{repo}/pulls]     (unchanged)
```

### The cells found while PLANNING round 5

Same provenance caveat `19-14` established — found while planning, not by an
audit.

```
exit=0  {git,push,--force,origin,main}              ARGV[git]: [push][--force][origin][main]   GENUINE
exit=0  {env,git} push --force origin main          ARGV[git]: [push][--force][origin][main]   GENUINE
exit=0  {g..g}it push --force origin main           ARGV[git]: [push][--force][origin][main]   GENUINE
exit=0  {g..g}{i..i}t push --force origin main      ARGV[git]: [push][--force][origin][main]   GENUINE
exit=0  {g..g..1}it push --force origin main        ARGV[git]: [push][--force][origin][main]   GENUINE
exit=0  {g..g}"it" push --force origin main         ARGV[git]: [push][--force][origin][main]   GENUINE
exit=0  {g..g}h pr create --title x                 ARGV[gh]:  [pr][create][--title][x]        GENUINE
exit=0  git {--no-pager,push} --force origin main   ARGV[git]: [--no-pager][push][--force]…    GENUINE
exit=0  git {stash,-q}                              ARGV[git]: [stash][-q]                     GENUINE
exit=0  git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}
                                       ARGV[git]: [push][origin][refs/…/w][--force][--force]   GENUINE
exit=0  g{i,i}t push --force origin main            ARGV[git]: [git][push]…                    COVERAGE
exit=0  {g,g}{i,i}{t,t} push --force origin main    ARGV[git]: [git]x7 [push]…                 COVERAGE
exit=0  {g{i,i}t,x} push --force origin main        ARGV[git]: [git][x][push]…                 COVERAGE
exit=0  "g"{i,i}"t" push --force origin main        ARGV[git]: [git][push]…                    COVERAGE
exit=0  g{it,x} push --force origin main            ARGV[git]: [gx][push]…                     COVERAGE
exit=0  {g,x}it push --force origin main            ARGV[git]: [xit][push]…                    COVERAGE
exit=0  {g,x}{h,h} pr create --title x              ARGV[gh]:  [gh][xh][xh][pr]…               COVERAGE
exit=0  git commit -m {a,b}                         ARGV[git]: [commit][-m][a][b]
exit=0  git config core.hooksPat? /tmp/x
exit=0  gh api repos/o/r/pul?s -f title=x                                empty walk
exit=0  git ~push --force origin main               ARGV[git]: [~push][--force][origin][main]  COVERAGE
exit=2  X=push,--force; IFS=,; git $X origin main   [envelope_assertion_failed]                COVERAGE
exit=0  git push --forc? origin refs/heads/gsd-auto/alpha/w                          T-19-96, registered
exit=2  git push --force origin refs/heads/gsd-auto/alpha/w  [force_push_blocked]    T-19-96 control
```

### Walked envelope listings, measured

```
refused  gh {pr,create} --title x                       ->  (the envelope root is empty)
counted  gh pr create --title x                         ->  alpha/pr-ledger.ndjson (112 bytes)
                                                            ledger lines found: 1
counted  gh api "repos/{owner}/{repo}/pulls" -f title=x ->  alpha/pr-ledger.ndjson (112 bytes)
         and the SECOND call in that same root          ->  exit 2, pr_cap_exceeded
                                                            alpha/pr-ledger.ndjson (224 bytes)
```

### `T-19-93`'s rows: the bar is COUNT

```
today: gh api repos/{owner}/{repo}/pulls -f title=x        -> exit 0, EMPTY walk   (RED)
today: (a second one in the same root)                     -> exit 0, EMPTY walk   (RED)
today: gh api "repos/{owner}/{repo}/pulls" -f title=x      -> exit 0, 1 ledger line (PASSES — positive control)
today: (a second quoted one in the same root)              -> exit 2, pr_cap_exceeded (PASSES)
today: gh api repos/{owner}/{repo}/pulls/7 -f body=x       -> exit 0, EMPTY walk   (PASSES — boundary)
today: gh api "repos/{owner}/{repo}/pulls/7" -f body=x     -> exit 0, EMPTY walk   (PASSES — boundary)
```

### `T-19-91` — the in-namespace `git push $REF` measurement

Reproduced in a purpose-built fixture on `gsd-auto/alpha/work` with a local bare
upstream, the repository passed to `guard_in` **explicitly** rather than taken
from the test process's working directory (`T-19-80`, the reason `19-14` declined
the pin — passing the root as a parameter removes the dependence, so the pin is
safe):

```
in namespace      git push $REF            -> exit 0                          <- audit 4 confirmed
in namespace      git push origin $REF     -> exit 2  push_outside_namespace  <- still fails closed
out of namespace  git push $REF            -> exit 2  push_outside_namespace
                  git reflog $S            -> exit 0
                  git reflog show $S       -> exit 0
                  git symbolic-ref $S      -> exit 0
                  git symbolic-ref HEAD $R -> exit 2  force_push_blocked
```

`T-19-91` stays OPEN at `high`. `git push` has `pre-push` behind it; `git reflog`
and `git symbolic-ref` have no `pre-push` and no `pre-commit` — git runs no hook
for either.

### `T-19-96` — registered, not fixed

```
exit=0  git push --forc? origin refs/heads/gsd-auto/alpha/w
exit=2  git push --force origin refs/heads/gsd-auto/alpha/w   [force_push_blocked]
```

Invariant under the working directory (measured identically with and without an
in-namespace project root, because the explicit refspec means no push context is
resolved). Registered because it was found while PLANNING and a plan cannot both
discover a threat and be the plan that measured it fail first. **Round discipline
is the reason, not blast radius** — `git push` does have `pre-push` behind it,
which `git stash` and `git update-ref` do not, but that is a narrowing rather
than a covering. Pinned at its measured verdict so a later change that reaches it
turns the pin red.

## Clause derivations for every pinned cost row

Written beside each row in the test file. `19-17`'s clauses: **1** — a named
decision word is not literal; **2a** — a segment of a brace-spliced simple
command resolves `Governed`/`NestedPayload`; **2b** — a word the splice can
PRODUCE has a governed basename.

| refused after 19-17 | clause | derivation | permitted twin (before AND after) |
|---|---|---|---|
| `git commit -m {a,b}` | 2a | the segment `git commit -m ` resolves `Governed`; its simple command is brace-spliced | `git commit -m ab` |
| `git add {src,tests}/x.rs` | 2a | the segment `git add ` resolves `Governed`; brace-spliced | `git add src/x.rs tests/x.rs` |
| `echo {git,x}` | 2b | products are `git` and `x`; `git`'s basename is governed, so an ungoverned `echo` is refused — clause 2(b)'s own over-refusal | `echo git` |
| `git config --get-regexp branch.*` | 1 | `config_key_operand_index` names `branch.*` a decision word; a glob makes it not literal | `git config --get-regexp 'branch.*'` |
| `git -c 'user.na*e=x' commit -m y` | 1 (textual) | `scan_leading`'s `-c` key check is a pure argv function and cannot see that quoting made the `*` literal. Cost disclosed and essentially zero: git config keys are `section.key` over alphanumerics, `.`, `-`, `_`, so no legal key can carry one | `git -c user.name="$NAME" commit -m x` |
| `git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}` | 2a | the segment resolves `Governed`; brace-spliced | `git push origin refs/heads/gsd-auto/alpha/w --force` (refused as `force_push_blocked`, before and after) |
| `git push {--force,origin} main` (in namespace) | 2a | the segment `git push` resolves `Governed`; brace-spliced | — |
| `git {--no-pager,push} --force origin main` | 2a | the segment `git` resolves `Governed`; brace-spliced | — |
| glob decision words (`git pus?`, `gh p? create`, `git config core.hooksPat?`, `gh api …/pul?s`, `git ~push`) | 1 | each slot is a decision word `19-14` already named through the scan the classifier itself runs | `git add src/*.rs`, `rg "x" src/*` |

**`ls {git,svn}-repo` is pinned PERMITTED**, and it is the control that
distinguishes a PRODUCT test from a NAME test: its products are `git-repo` and
`svn-repo`, neither of whose basename is governed. A rule that refused it would
be testing for a mention, and the whole finding that reshaped this round is that
a mention test is one slot away from the class.

## The two shapes measured but deliberately NOT pinned post-fix

* **`FOO={a,b} git status` — measured exit 2 `envelope_assertion_failed` TODAY.**
  It is an **INTERACTION, not a new cost**: `}` is a word-splitting closer, the
  segment `git status` reports `head_is_command_position == false`, and Rule B
  refuses it. Nothing this round adds is load-bearing for this line. Pinned at its
  true identifier in
  `the_assignment_value_splice_is_already_refused_for_another_reason_and_is_an_interaction`,
  the shape `19-15` established with
  `the_severed_spelling_that_binds_a_whole_key_is_already_refused_for_another_reason`.
  Its would-be twin `FOO=ab git status` is measured PERMITTED and pinned beside
  it.
* **`rg "git status" {src,tests}` — measured exit 0 TODAY, NOTHING asserted
  post-fix.** Whether clause 2(a) reaches it depends on whether `resolve_program`
  answers `NestedPayload` for the quoted payload inside a marked command, which
  this plan cannot state with certainty — and a plan that measures PRE-fix has no
  method that would catch a wrong post-fix expectation. `19-17` measures it after
  the rule exists and pins it with the clause that produced it. Its unambiguous
  sibling `rg "git status" src/` IS pinned permitted.

## Per-class generated-case counts, counted while generating

5180 cases from 37 refused bases; **3308** carry an unreadable class. Floor is 20
per class.

| class | generated cases |
|---|---|
| whole-word splice | 847 |
| concatenated splice | 1255 |
| range | 420 |
| multi-expansion word | 280 |
| literal brace pair | 140 |
| glob | 1202 |
| tilde | 115 |

The predicates are asserted degenerate-proof in
`the_corpus_can_draw_every_one_of_the_seven_unreadable_classes`: `{a,b}` IS a
whole-word splice and is NOT concatenated, NOT a range and NOT multi-expansion;
`{a,b} {c,d}` (two words) is NOT multi-expansion; `{a,b}{x}` (second pair
literal) is NOT multi-expansion; `{g..g}{i..i}t` IS; `HEAD@{0}` is a literal
brace pair and is not a range.

## The renamed floor

| | |
|---|---|
| old name | `every_alphabet_this_plan_widens_can_draw_an_expansion_metacharacter` |
| new name | `every_alphabet_this_phase_widens_can_draw_an_expansion_metacharacter` |
| reason | Its body is unchanged and still covers the six alphabets audit 3's axis table named, including `SHELL_LAYERS`. "this plan" read as plan 19-15; the floor now sits beside a second, wider floor added by plan 19-16, so the name had to say which scope it means. The wider floor is `every_alphabet_this_round_widens_can_draw_a_word_the_guard_cannot_read`, which is a strengthening beside it rather than a replacement of it |

No `MIN_*` constant changed value, no alphabet entry was deleted or reordered, no
existing property was modified. `git diff` shows five deleted lines in
`tests/envelope_wrapper_class.rs` and all five are benign: the `BTreeSet` import
(extended to `{BTreeMap, BTreeSet}`), two arithmetic comments (`15 x 140` → `37 x
140`, `9 x 120` → `11 x 120`), the floor rename, and `_ =>` becoming `2 =>` in
`expansion_spelling` with the identical body plus new arms 3 and 4 that only the
new properties draw.

## Deviations from Plan

### 1. [Rule 3 — blocking] The quoted-run spellings stay OUT of `REFUSED_BASES`

* **Found during:** Task 2.
* **Issue:** The plan (and plan-check re-check 2) direct
  `"g"{i,i}"t" push --force origin main` and `{g..g}"it" push --force origin main`
  into `REFUSED_BASES`. Every base in that alphabet is re-quoted by five
  `ShellLayer`s, two of which wrap the whole payload — `sh -c '…'` and
  `bash -lc "…"`. A base carrying `"` produces illegal shell under `BashDashLC`,
  and the file's own doc says a generator that emitted illegal shell "would be
  exercising the splitter's error path while claiming to exercise the class".
* **Fix:** Both spellings are enumerated rows in
  `tests/envelope_literal_decision.rs` instead, where no outer layer re-quotes
  them, and the CONCATENATED and MULTI-EXPANSION classes they belong to are drawn
  in `REFUSED_BASES` by quote-free spellings.
* **Commit:** `242b755`.

### 2. [Rule 3 — blocking] `SEVERED_PREFIXES` not widened; a new alphabet added instead

* **Found during:** Task 2.
* **Issue:** `a_governed_program_behind_a_severed_prefix_is_refused_wherever_the_split_falls`
  asserts `live_expansion_cases == cases` — EVERY generated severed case must
  carry a `$` or a backtick. Brace entries carry neither, so adding them turned
  that existing floor red (67 of 80). Lowering it is forbidden.
* **Fix:** A new `SEVERED_BRACE_PREFIXES` alphabet with a property of its own,
  `a_governed_program_behind_a_brace_severed_prefix_is_refused_wherever_the_split_falls`,
  which counts the whole-word and concatenated classes separately and passes
  today. `SEVERED_PREFIXES` and its property are byte-identical.
* **Commit:** `242b755`.

### 3. [Rule 2 — missing critical correctness] No glob entry in `EXPANSION_WRAPPERS`

* **Found during:** Task 2.
* **Issue:** The plan directs "brace-expansion and glob wrapper prefixes" into
  `EXPANSION_WRAPPERS`. That property's verdict comes from `resolve_program` step
  5's prefix rule, which decides on `Token.expansion` — and `19-17` states
  explicitly that its new literalness bit "is not a second `expansion`, which
  stays exactly as it is because `resolve_program` steps 3 and 5 decide on it and
  this plan does not move them". A glob wrapper prefix would therefore be pinned
  at a refusal `19-17`'s rule cannot produce, which is the one error a
  measure-pre-fix plan has no method to catch.
* **Fix:** Only brace-expansion prefixes added, with the reason written into the
  alphabet's doc comment so a later round does not re-add the glob without
  re-deriving it.
* **Commit:** `242b755`.

### 4. [Rule 3 — blocking] The flag-slot splice excluded from `REFUSED_BASES`

* **Found during:** Task 2.
* **Issue:** `git push {--force,origin} main` makes `push_needs_resolved_dests`
  answer true, so the guard shells out to `git` in the caller's working
  directory. `REFUSED_BASES`' own doc excludes exactly that shape (`T-19-80`), and
  140 wrappings would consult a repository 140 times.
* **Fix:** `git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}` carries the
  concatenated flag-slot class into `REFUSED_BASES` instead — explicit
  in-namespace refspec, so no push context is resolved — and the whole-word
  spelling is an enumerated row pinned in the in-namespace configuration with the
  repository passed explicitly.
* **Commit:** `8866012`, `242b755`.

### 5. [Rule 1 — bug] One new clippy warning fixed before commit

* **Found during:** Task 2. `UNREADABLE_CLASSES: &[(&str, fn(&str) -> bool)]`
  tripped `clippy::type_complexity`, taking `--all-targets` from four warnings to
  five. Added a `type UnreadableClass` alias; back to the four pre-existing.
* **Commit:** `242b755`.

## Findings — rows that were not what the plan expected

Both are recorded in `19-SECURITY.md`'s appended plan-19-16 execution record and
in the test file's own comments.

### A. A comma list produces N WORDS, so most "concatenated splice" spellings are COVERAGE

The plan describes eight concatenated/multi-expansion/nested/quoted spellings as
"all eight assembled by bash into a real force push". Measured under the shims,
that is true of **four** and false of the rest. A brace expansion carrying a comma
list of N alternatives produces N words, so a concatenated comma splice in a
PROGRAM slot always leaves a surplus word in the subcommand slot:

```
g{i,i}t push --force origin main   ->  git git push --force origin main
                                       real git: git: 'git' is not a git command.
{g{i,i}t,x} push --force …         ->  git x push --force …
                                       real git: git: 'x' is not a git command.
{g,x}{h,h} pr create --title x     ->  gh gh xh xh pr create --title x
                                       real gh:  unknown command "gh" for "gh"
```

Only a RANGE whose endpoints are equal (`{g..g}`, `{i..i}`, `{g..g..1}`) produces
exactly one word, so only the range spellings genuinely run
`git push --force origin main`.

**Nothing was dropped and nothing was quietly re-labelled.** The comma spellings
stay in the corpus — the guard is blind to them identically, one character
separates them from the range spellings, and the rule that reaches one reaches
the other — but they are labelled **COVERAGE** row by row with the printed argv,
so a later reader cannot mistake one for a reproduced harm. Two genuine
comma-based reproducers were found and added to compensate:
`{env,git} push --force origin main` (the first product is a wrapper, so the
second is the program it runs) and `git {--no-pager,push} --force origin main`
(the surplus word is a valid git global option). Neither the range class, the
multi-expansion class nor the concatenated class loses a genuine reproducer.

### B. `git push {--force,origin} main` is CWD-DEPENDENT

Not permitted today as the plan states — the answer depends on the repository:

```
project root outside the namespace (this checkout, on master)
  -> exit 2, push_outside_namespace  — the braces fragment the line into a
     `git push` with no refspec, and the no-refspec arm refuses it for a reason
     that has nothing to do with the splice
project root INSIDE the namespace (refs/heads/gsd-auto/alpha/work, upstream
configured — the state a driven run is DESIGNED to be in)
  -> exit 0
```

So it is a live bypass in the configuration the envelope exists for, and out of
namespace it is MASKED by an unrelated arm. Pinned in the in-namespace
configuration with the repository passed as an explicit parameter to `guard_in`,
and pinned from the other side out of namespace with an assertion that accepts
either refusal identifier so `19-17` may strengthen it. This is the same shape as
audit 4's own `T-19-91` correction, one arm over.

### C. No row failed to reproduce, and no row was discarded

All fourteen audit-4 rows reproduced at their recorded verdicts. No candidate had
to be discarded for the shell not running it at all (audit 4 discarded three that
way); the eleven COVERAGE rows do reach the governed program and are refused by
the same rule — the weaker claim is only about the destructiveness of the final
argv.

## Commits

| # | sha | what | `src/` hunks |
|---|---|---|---|
| 1 | `8866012` | `test(19-16)`: the reproducers, RED (20 pass / 21 fail) | **0** |
| 2 | `242b755` | `test(19-16)`: the widened alphabets, RED (20 pass / 5 fail) | **0** |
| 3 | `6827d9e` | `docs(19-16)`: the execution record and the `T-19-96` registration | **0** |

`git diff --stat HEAD~3..HEAD` touches exactly the four files in
`files_modified` and nothing else — not `src/`, not `Cargo.toml`, not
`Cargo.lock`, not `tests/envelope_expansion_slots.rs`, not
`tests/envelope_command_position.rs`. The `19-SECURITY.md` and
`deferred-items.md` diffs are pure appends (255 and 86 insertions, **zero**
deletions), so no audit table, Security Audit Trail entry, Accepted Risks Log
row, sign-off or earlier appended subsection was touched.

## What remains uncovered

1. **`T-19-86`** — OPEN at `high`, by explicit user scoping decision. A governed
   program's own operand carrying a whole command line. All four rows still exit
   0; the pin is unmodified and green.
2. **`T-19-91`** — OPEN at `high`. Measured wider here; the record correction is
   handed to `19-17`. `reflog` and `symbolic-ref` have no second carrier.
3. **`T-19-92`, `T-19-93`, `T-19-94`, `T-19-95`** — all OPEN. This plan closes
   nothing; it produces the evidence `19-17` will be certified against.
4. **`T-19-96`** — registered, measured, pinned, not fixed.
5. **`T-19-74`** — accepted (AR-19-10); core rows frozen and re-measured green.
6. **`T-19-84`, `T-19-85`, and `T-19-61` … `T-19-73`** — open, unaccepted,
   untouched. `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not
   opened.
7. **The `resolve_program_with_head` wildcard arm** audit 4 flagged as a
   regression surface — not a live bypass, not registered as a threat, and
   `19-17`'s work.

**`/gsd-secure-phase 19` is NOT cleared by this plan or by `19-17`.**

## Self-Check: PASSED

* `tests/envelope_literal_decision.rs` — FOUND
* `tests/envelope_wrapper_class.rs` — FOUND
* `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md` — FOUND
* `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` — FOUND
* commit `8866012` — FOUND
* commit `242b755` — FOUND
* commit `6827d9e` — FOUND
