---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 17
subsystem: envelope
tags: [security, gap-closure, tokenizer, brace-expansion, inversion]
status: complete
requires:
  - "19-16's corpus, committed RED with zero `src/` hunks"
  - "19-15's Rule B and its severed-prefix geometry"
  - "19-14's decision region, slot-exact and unmoved"
provides:
  - "`Token.literal` — positive evidence that the shell hands a word over unchanged"
  - "the tokenizer's three-way `{` classification and the whole-word product scan"
  - "clause 2 in an EXHAUSTIVE `resolve_program_with_head`, no wildcard arm"
  - "`T-19-93` closed by COUNT with neither forge scan changed"
affects:
  - "/gsd-secure-phase 19, which is NOT cleared — T-19-86 and T-19-91 stay open at high"
tech-stack:
  added: []
  patterns:
    - "invert an enumeration into positive evidence when five rounds of enumerating have not converged"
    - "compute a splice's PRODUCTS over the whole word, never per-`{`, and join literal runs quote-removed"
    - "take a nested case as a fail-closed TRIGGER rather than as a recursion"
    - "make a post-filter exhaustive so a future variant is an E0004 rather than a quiet permit"
key-files:
  created:
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-17-SUMMARY.md
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - tests/envelope_literal_decision.rs
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The rule is INVERTED rather than extended: a decision word must be provably LITERAL, so anything not provably literal refuses"
  - "Clause 2(b) reads what a splice PRODUCES, never what its alternatives are called, and the products are composed over the WHOLE WORD with literal runs joined quote-removed"
  - "A nested `{` inside an alternative is an unenumerable TRIGGER, not a recursion — a fail-closed refusal needs no depth, no bound and no second reading"
  - "T-19-93 is closed in the tokenizer's literal-brace branch and NOT by a placeholder tolerance in the forge scans"
  - "`api_flag_ness_is_unreadable` was widened to the same character class the bit covers — a finding of this execution, caught by the generative forge-slot property after clauses 1 and 2 had landed"
  - "The rule was NOT weakened to keep a stale pre-fix handoff guard green; the guard was corrected and the deviation reported"
metrics:
  duration: "~1 session"
  completed: 2026-08-29
actuals:
  tokens: 96000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 17: The Inverted Rule — Summary

A governed segment's decision words must be **LITERAL** — the shell hands them
to the program byte-identically to how they are written — established by
positive evidence collected while the word is consumed, with a second clause at
the simple-command level for brace expansion, which is not a word-level fact.

## Read this first: the gate is NOT cleared

**`/gsd-secure-phase 19` is NOT cleared by this plan.**

* **`T-19-86`** — OPEN at `high`, by explicit user scoping decision. Untouched
  and unremediated; all four rows re-measured at exit 0 against the built binary.
* **`T-19-91`** — OPEN at `high`. Its code-side RECORD is corrected here; the
  remedy is unchanged and no decision-operand rule was added for `reflog`,
  `symbolic-ref` or `push`.
* **`T-19-96`** — registered open, pinned at its measured verdict, **not fixed**.
* **`T-19-74`** — accepted (AR-19-10); core rows re-measured permitted.
* **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open, unaccepted, untouched.
  `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not opened.

Only the **WRAPPER-OPERAND sub-class** of `T-19-60` is closed. `T-19-86` and
`T-19-91` are both sub-classes of it and both remain OPEN at `high`.

## The RED confirmation, before any production line moved

`rtk proxy cargo test --no-fail-fast` at `09e83bd`, `19-16`'s post-state:

```
passed=1554 failed=26 ignored=13 total=1580
```

Exactly the total `19-16-SUMMARY.md` records, in the 1554/26 flake mode (the
`driver_reattach` pair passing). **All 26 failing names are the ones `19-16`
lists, and no third kind of name appeared.** Per-binary:

```
Running tests/envelope_literal_decision.rs -> FAILED. 20 passed; 21 failed
Running tests/envelope_wrapper_class.rs    -> FAILED. 20 passed;  5 failed
Running tests/envelope_expansion_slots.rs  -> ok.     32 passed;  0 failed
Running tests/envelope_command_position.rs -> ok.     18 passed;  0 failed
Running tests/driver_reattach.rs           -> ok.      3 passed;  0 failed
```

The 21 in `envelope_literal_decision`, each prefixed `after_19_17_`:

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

The 5 in `envelope_wrapper_class` — the three generative properties at their
shared unwrapped floor over the widened `REFUSED_BASES`, plus the two new forge
properties:

```
a_forge_decision_slot_carrying_a_splice_or_a_glob_is_refused_in_every_generated_slot
a_gh_placeholder_endpoint_is_permitted_and_counted_in_every_generated_placement
a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position
a_wrapper_prefix_carrying_an_expansion_is_refused_in_every_generated_position
the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command
```

**Every one of the 26 is GREEN after.** That transition, from red in a commit
with zero `src/` hunks to green here, is the one number a run that did nothing
cannot forge: a run that changed nothing fails twice over — the total is
unchanged AND the named rows are still failing.

## The final gate

`rtk proxy cargo test --no-fail-fast`:

```
passed=1584 failed=0 ignored=13 PASSED+FAILED=1584
full_suite_exit=0
```

**The gate arithmetic, stated and checked rather than assumed.** Turning a
failing test green leaves `passed + failed` unchanged, because a red test RAN.
The only source of increase is NEW `#[test]` functions, and `git diff` over
`09e83bd..HEAD` counts exactly **4**:

| new `#[test]` fn | file | why it is mandatory |
|---|---|---|
| `the_tokenizers_three_way_brace_classification_and_its_per_word_products` | `src/envelope/policy.rs` | the brace-classification table with its per-WORD PRODUCTS column |
| `the_brace_splice_fact_reaches_every_segment_of_the_simple_command` | `src/envelope/policy.rs` | the splice mark asserted at the `Segment` level, not only through an exit code |
| `the_marked_payload_splice_is_refused_by_clause_2a_and_that_is_a_pinned_cost` | `tests/envelope_literal_decision.rs` | a deferred row, measured after the rule exists |
| `the_assignment_value_splice_stays_an_interaction_after_the_rule_and_not_a_new_cost` | `tests/envelope_literal_decision.rs` | the other deferred row, asserted over Rule B's mechanism |

`1580 + 4 = 1584`, and the observed number is 1584. The identity holds.

**Failures: 0.** The documented-flaky `driver_reattach` pair passed in this run
and was neither fixed nor touched.

### Per-binary counts

| binary | result |
|---|---|
| `--test envelope_literal_decision` | ok. **43** passed; 0 failed |
| `--test envelope_wrapper_class` | ok. **25** passed; 0 failed |
| `--test envelope_expansion_slots` | ok. **32** passed; 0 failed |
| `--test envelope_command_position` | ok. **18** passed; 0 failed |
| `--lib envelope::policy` | ok. **63** passed; 0 failed |
| `--lib envelope` | ok. **186** passed; 0 failed |

`cargo build` and `cargo clippy -- -D warnings` exit 0. `cargo clippy
--all-targets` reports the same **four** pre-existing warnings (three in
`src/browser.rs:155-157`, one in `src/project_creator.rs:146`) and no fifth. No
crate was added; neither `Cargo.toml` nor `Cargo.lock` was touched (`T-19-SC`).

## The definition of LITERAL, as implemented

`Token.literal` is **positive evidence** that the shell hands the word to the
program byte-identically to how it is written. It is collected by
`policy::tokenize` while the word is consumed and is **never** a test on the
recovered text — that text has had its quoting removed, and in it
`gh api "repos/{owner}/{repo}/pulls"` (literal, and counted before this plan) is
indistinguishable from a word the shell rewrites.

Cleared when the tokenizer consumes, **outside quotes**:

| class | characters | why it is not literal |
|---|---|---|
| expansion | `$`, `` ` `` | parameter, command and arithmetic expansion, and every `$IFS` re-split of the result |
| pathname | `*`, `?`, `[` | the result depends on the working directory, so it is unknowable **whether or not a file matches today** — a precondition an agent satisfies with `touch push` in the same tool call |
| tilde | `~` | the result depends on the passwd database of the machine the command will run on |
| brace | a `{`…`}` pair classified as an EXPANSION, or a `{` with no match | bash splices its alternatives back into the enclosing command, so the word never exists as written |

Inside DOUBLE quotes, **only** `$` and `` ` `` clear it.

**Deliberately NOT cleared for three things:**

1. anything inside **single quotes**;
2. a **backslash-escaped** character — escaping is exactly what makes a character
   literal;
3. a `{`…`}` pair with **no comma and no range**, which bash passes through
   unchanged (`printf "[%s]" repos/{owner}/{repo}/pulls` prints it byte for byte,
   measured).

It is **not a second `expansion`**: `Token.expansion` is unchanged, because
`resolve_program` steps 3 and 5 decide on it and this plan did not move them.

## Every `19-16` RED row, before and after

Measured against `./target/debug/gsd-meta-manager envelope guard alpha`, one
fresh `GSD_MM_ENVELOPE_ROOT` per row, the envelope directory walked afterwards
so "no ledger line" is **observed** rather than derived.

### Clause 2(a) — a segment resolves `Governed`, and the command is brace-spliced

| command | before | after | reason id | walk |
|---|---|---|---|---|
| `git {-c,core.hooksPath=/dev/null,push,--force,origin,main}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {push,--force} origin main` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {update-ref,-d,refs/heads/main}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {config,core.hooksPath,/tmp/x}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {stash,-q}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {--no-pager,push} --force origin main` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `gh {pr,create} --title x` | 0 | **2** | `envelope_assertion_failed` | empty |
| `gh {api,repos/o/r/pulls} -f title=x` | 0 | **2** | `envelope_assertion_failed` | empty |
| `glab {mr,create} --title x` | 0 | **2** | `envelope_assertion_failed` | empty |
| `bash -lc "git {push,--force} origin main"` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git push {--force,origin} main` (in namespace) | 0 | **2** | `envelope_assertion_failed` | empty |

The last row was **cwd-dependent** before — exit 0 in namespace, exit 2 out of it
under an unrelated arm. It is now refused before any push context is resolved, so
the verdict no longer depends on the repository at all.

The `bash -lc` row arrives through the `NestedPayload` re-split: its braces are
LITERAL until the payload is re-split, so the splice only appears at that point
and the facts are threaded through the same `split_segments_with_heads` call.

### Clause 2(b) — NOTHING in the line resolves `Governed`

The head word is `it`, `g`, `t`, or a comma list naming nothing. No
alternative-NAME test reaches any of these, and the composed word never exists as
a token.

| command | before | after | how it is reached |
|---|---|---|---|
| `{git,push,--force,origin,main}` | 0 | **2** | products contain `git` |
| `{env,git} push --force origin main` | 0 | **2** | products contain `git` |
| `{g..g}it push --force origin main` | 0 | **2** | concatenated: product `git` |
| `g{i,i}t push --force origin main` | 0 | **2** | concatenated: product `git` |
| `g{it,x} push --force origin main` | 0 | **2** | concatenated: products `git`, `gx` |
| `{g,x}it push --force origin main` | 0 | **2** | concatenated: products `git`, `xit` |
| `{g..g}{i..i}t push --force origin main` | 0 | **2** | **multi-expansion**, composed ACROSS both `{`s |
| `{g,g}{i,i}{t,t} push --force origin main` | 0 | **2** | **multi-expansion**: 8 products, all `git` |
| `{g..g..1}it push --force origin main` | 0 | **2** | **UNENUMERABLE**: the increment range |
| `{g{i,i}t,x} push --force origin main` | 0 | **2** | **UNENUMERABLE**: a nested `{` in an alternative |
| `"g"{i,i}"t" push --force origin main` | 0 | **2** | runs joined **quote-removed** |
| `{g..g}"it" push --force origin main` | 0 | **2** | runs joined **quote-removed** |
| `{g..g}h pr create --title x` | 0 | **2** | product `gh`; walk empty |
| `{g,x}{h,h} pr create --title x` | 0 | **2** | product `gh`; walk empty |

Rows 7–8 are why the product scan is **whole-word rather than per-`{`**: a
per-`{` computation answers `g` and `it` for `{g..g}{i..i}t` — neither governed,
both sets enumerating cleanly — and the command would be permitted. Row 10 is why
a nested `{` is a **fail-closed trigger rather than a recursion**. Rows 11–12 are
why the literal runs are **quote-removed before they are joined**: joined as
written, the product of `"g"{i,i}"t"` reads `"g"i"t"`, whose basename is not
governed.

### Clause 1 — a decision word that is not literal

| command | before | after | which decision word |
|---|---|---|---|
| `git pus? --force origin main` | 0 | **2** | the git verb |
| `git ?ush --force origin main` | 0 | **2** | the git verb |
| `git stas?` | 0 | **2** | the git verb (no second carrier) |
| `touch push && git pus? --force origin main` | 0 | **2** | the git verb of the SECOND segment |
| `gh p? create --title x` | 0 | **2** | a forge subcommand word; walk empty |
| `git config core.hooksPat? /tmp/x` | 0 | **2** | the `git config` key operand |
| `gh api repos/o/r/pul?s -f title=x` | 0 | **2** | the `gh api` endpoint; walk empty |
| `gh api repos/o/r/pulls -? title=x` | 0 | **2** | a `gh api` word whose flag-ness decides |
| `git ~push --force origin main` | 0 | **2** | the git verb — **CLASS COVERAGE**, not a measured bypass |

## `T-19-93` — closed by COUNT, and the evidence is a walked LEDGER LISTING

```
gh api repos/{owner}/{repo}/pulls -f title=x   (one fresh envelope root, walked)
  call 1 -> exit 0
    alpha/pr-ledger.ndjson (112 bytes)      ledger lines found: 1
  call 2 -> exit 2  reason pr_cap_exceeded
    alpha/pr-ledger.ndjson (224 bytes)      ledger lines found: 2

gh api "repos/{owner}/{repo}/pulls" -f title=x        (the POSITIVE control)
  call 1 -> exit 0    1 ledger line
  call 2 -> exit 2    pr_cap_exceeded

gh api repos/{owner}/{repo}/pulls/7 -f body=x         (the BOUNDARY)
  call 1 -> exit 0    ledger lines found: 0
  call 2 -> exit 0    ledger lines found: 0
```

Before this plan the unquoted form was exit 0 with an **empty walk on both
calls** — the SAFE-06 cap BYPASSED rather than exceeded, with no second carrier
(`T-19-35`). It now behaves byte-for-byte like the quoted spelling, and the
`…/pulls/7` boundary still writes nothing, so the tolerance did not degrade into
"any endpoint with braces counts".

### Why it was closed in the TOKENIZER rather than in the forge scans

**Neither forge scan was changed.** The rejected alternative — teaching
`scan_gh_api`/`endpoint_is_pulls` about `{owner}`/`{repo}` — is recorded in
`endpoint_is_pulls`' own doc with three reasons a later reader can check:

1. **It is where the defect is.** The braces are literal in bash; the tokenizer
   was wrong about them, not the forge scans. A tolerance in the scans is a
   second consumer compensating for a splitter that mangles the word — the shape
   this phase has produced four times.
2. **It generalises.** `endpoint_is_pulls` is one consumer of one word. The same
   literal spelling can appear in a git verb, a `config` key operand and a `-c`
   assignment; a scan tolerance fixes one cell and leaves the rest one slot over.
3. **It is free.** The tokenizer had to learn to tell a brace expansion from a
   brace pair anyway, for `T-19-92`. With that in place `endpoint_is_pulls`
   already answers `true` for `repos/{owner}/{repo}/pulls` with no change at all.

## What the inversion NEWLY REFUSES

Every cost pinned beside its PERMITTED twin **and** the clause that produces it.
Both sides measured against the built binary.

| refused after 19-17 | clause | permitted twin (before AND after) |
|---|---|---|
| `git commit -m {a,b}` | 2a | `git commit -m ab` — exit 0 |
| `git add {src,tests}/x.rs` | 2a | `git add src/x.rs tests/x.rs` — exit 0 |
| `rg "git status" {src,tests}` | 2a | `rg "git status" src/` — exit 0 |
| `echo {git,x}` | **2b's own over-refusal** | `echo git` — exit 0 |
| `git config --get-regexp branch.*` | 1 | `git config --get-regexp 'branch.*'` — exit 0 |
| `git -c 'user.na*e=x' commit -m y` | 1 (textual) | `git -c user.name="$NAME" commit -m x` — exit 0 |
| `gh api repos/o/r/pulls -? title=x` | 1 | `gh api repos/o/r/pulls -f title="$T"` — exit 0 |

**`ls {git,svn}-repo` is PERMITTED — measured exit 0 — and it is the control that
shows clause 2(b) reads PRODUCTS rather than NAMES.** Its products are `git-repo`
and `svn-repo`, neither of whose basename is governed. A rule that refused it
would be testing for a mention, and the whole finding that reshaped this round is
that a mention test is one slot away from the class.

The `-c` key-half cost is **textual and disclosed**: `scan_leading` is a pure
argv function that sees the word after the tokenizer removed its quoting, so it
cannot tell that the quoting made the `*` literal. The false-positive cost is
essentially zero, and the reason is stated rather than assumed — a git config key
is `section.key` over alphanumerics, `.`, `-` and `_`, so no legal key can carry
one of these characters.

### The whole allow corpus, re-measured at exit 0

`echo {a,b}`, `mkdir -p {src,tests}`, `cp x{,.bak}`, `ls *.rs`, `rg "x" src/*`,
`git add src/*.rs`, `cd ~/projects`, `git commit -m "use ${HOME} here"`,
`gh pr create --title 'fix $PATH handling'`, `git config user.email "$EMAIL"`,
`echo $(git rev-parse HEAD)`, `ROOT=$(git rev-parse --show-toplevel)`,
`{ git status; }`, `( git status )`, `(git status)&&git fetch origin`,
`git log -1 HEAD@{0}`, `git reflog show HEAD@{0}`,
`env $X push --force origin main` and `X=git; env $X push --force origin main`.

The grouping refusals still fire, which is what shows `{ cmd; }` and `( cmd )`
were discriminated rather than deleted: `{ git push --force origin main; }`,
`( git push --force origin main )` and `git reflog delete HEAD@{0}` are all exit
2 under `force_push_blocked`. `SEPARATORS` is byte-for-byte unchanged and `(`/`)`
were not touched at all.

## The two rows `19-16` deferred, measured HERE with their derivations

| row | pre-fix | post-fix | classification and derivation |
|---|---|---|---|
| `FOO={a,b} git status` | 2 `envelope_assertion_failed` | 2 `envelope_assertion_failed` | **INTERACTION, not a new cost.** Rule B's geometry still holds — `}` is a word-splitting closer, so the segment `git status` reports `head_is_command_position == false` — and clause 2(a) reaches the same segment because the command is brace-spliced. Asserted over the MECHANISM as well as the verdict, because a verdict alone cannot tell "still an interaction" from "newly a cost". Its twin `FOO=ab git status` stays permitted |
| `rg "git status" {src,tests}` | 0 | **2** `envelope_assertion_failed` | **A NEW COST, by clause 2(a).** The segment `rg "git status"` resolves `NestedPayload` and its simple command is brace-spliced. It is the widest cost this round adds: bash runs `rg "git status" src tests`, in which nothing governed executes at all. Its twin `rg "git status" src/` stays permitted |

## The exhaustive post-filter, with its five arms named

`resolve_program_with_head` closes the fail-open seam audit 4 flagged but did not
register. It was `Governed | NestedPayload => Refuse, other => other`; a future
variant meaning "this segment reaches a program the envelope governs" would have
compiled, passed a severed head silently and turned no test red.

1. **`Governed`** — refuse on a severed head (Rule B, detail unchanged) **or** on
   a brace-spliced simple command (clause 2a).
2. **`NestedPayload`** — the same, and this is the arm that reaches
   `rg "git status" {src,tests}`.
3. **`NoProgram`** — pass through, **unless** clause 2(b) fires.
4. **`Ungoverned`** — pass through, **unless** clause 2(b) fires. This is the arm
   the whole PRODUCE class arrives on, since nothing in it resolves `Governed`.
5. **`Refuse`** — passes through WHOLE, keeping its own more specific identifier,
   so step 1's `HookBypassBlocked` is never overwritten by this one.

**No wildcard arm.** A sixth `ProgramResolution` variant is now an E0004 — the
discipline `T-19-45` already establishes for `PermissionMode`. Clause 2 is folded
into this one filter rather than added beside it, because two post-filters over
one resolution are two things to keep in step.

## The corrected `T-19-91` statement — the threat stays OPEN at `high`

`resolve_program`'s fourth residual bullet argued that `push`'s "refspec operand
already fails CLOSED". That is corrected in the code.

* The **narrow** claim is RIGHT and was re-measured: `git push origin $REF` is
  refused at exit 2, because an unreadable refspec does not carry the namespace
  prefix. That claim is untouched.
* The **bare** `git push $REF` is a different shape. `push_needs_resolved_dests`
  answers true for it, so its verdict is resolved from a repository and is
  **cwd-dependent — exit 0 in a repository whose current branch is inside the
  envelope's namespace**, which is the state a driven run is designed to be in,
  and exit 2 (`push_outside_namespace`) elsewhere for an unrelated reason.

So `classify_push`'s refspec operand is a **THIRD arm of `T-19-91`'s shape**
beside `classify_reflog`'s and `classify_symbolic_ref`'s. **`T-19-91` is not
closed, not renumbered and not re-scoped**, and the second-carrier statement is
unweakened: `push` has `pre-push` behind it while `reflog` and `symbolic-ref`
have no `pre-push` and no `pre-commit` — git runs no hook for either. The operand
rows are re-measured unchanged: `git reflog $S`, `git reflog show $S` and
`git symbolic-ref $S` all exit 0; `git symbolic-ref HEAD $R` exits 2.
`resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover` is
green — the `pub fn resolve_program(` anchor and the bullet structure are intact.

## Rule B is still load-bearing, asserted mechanically

Case 1 of the three-way `{` classification — a `{` immediately preceded in-word
by an UNQUOTED `$` — keeps today's behaviour byte-for-byte, and it exists for
exactly this reason. Fold it into the literal-brace case and `${C}_COUNT` becomes
one word: `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin` would be refused by
`resolve_program` step 5's prefix rule instead of by Rule B, every verdict pin in
the suite would stay green, and Rule B would quietly be dead code.

`rule_b_still_reports_a_severed_head_as_not_a_command_position` asserts over
`policy::split_segments_with_heads` directly rather than over an exit code, and
it is **green and unmodified**. The `${` case was not folded into the
literal-brace case.

## Commits

| # | sha | what | files |
|---|---|---|---|
| 1 | `958b7a1` | `feat(19-17)`: the three-way `{`, the literalness bit, the whole-word product scan | `src/envelope/policy.rs` |
| 2 | `d458e69` | `feat(19-17)`: the inverted rule, clause 2 in an exhaustive post-filter | `src/envelope/{policy,hooks}.rs`, `tests/envelope_literal_decision.rs`, `tests/envelope_wrapper_class.rs` |
| 3 | `2b0a901` | `docs(19-17)`: the `T-19-91` record correction and the execution record | `src/envelope/policy.rs`, `19-SECURITY.md`, `deferred-items.md` |

`git diff --stat 09e83bd..HEAD` touches exactly the six files in
`files_modified` plus the two test files, and nothing else — **not `Cargo.toml`,
not `Cargo.lock`**. The `19-SECURITY.md` and `deferred-items.md` diffs are
**pure appends** (299 and 77 insertions, **zero** deletions), so no audit table,
Security Audit Trail entry, Accepted Risks Log row, sign-off or earlier appended
subsection was touched.

At the end of commit 1 the suite was still red, as expected — no rule read the
new bit yet — at 22 passed / 19 failed in `envelope_literal_decision` and 21
passed / 4 failed in `envelope_wrapper_class`. The tokenizer alone turned the
three `T-19-93` counting rows green, which is exactly the plan's claim that
`T-19-93` is closed by the literal-brace branch and by nothing else.

## Deviations from Plan

### 1. [Rule 2 — missing critical correctness] `api_flag_ness_is_unreadable` had to be widened

* **Found during:** Task 2, by the generative property
  `a_forge_decision_slot_carrying_a_splice_or_a_glob_is_refused_in_every_generated_slot`,
  **after** clauses 1 and 2 had landed and every enumerated row was green.
* **Issue:** `gh api repos/o/r/pulls -? title=x` was still permitted at exit 0
  with an EMPTY walk — the SAFE-06 cap bypassed rather than exceeded. The plan
  states `api_flag_ness_is_unreadable` is UNCHANGED, but that predicate is a
  textual INDEX-SELECTION clause that still knew only `$` and a backtick, so a
  `?` or a `{` in a flag marker put the word **in no part of the region at all**
  — the closure never got an index to read the bit at.
* **Fix:** widened to the same character class the bit covers, for the identical
  reason the plan itself gives for widening `scan_leading`'s `-c` key half; the
  two are the same key-half readability test and leaving one behind would make
  them disagree. It still only selects an index — the readability question is
  answered by `Token.literal` in the one closure, so a QUOTED `-'*'` is named
  here and then passes there.
* **Not a second reading site**, and the region did not move: the same words are
  in it, named by the same predicate.
* **Commit:** `d458e69`.

### 2. [FINDING — reported, not laundered] `MIN_UNREADABLE_FORGE_SLOT_CASES` was unreachable by construction

* **Found during:** Task 2, once all 40 generated cases passed and execution
  reached the floor assertion for the first time.
* **Issue:** `19-16` set the floor at 50 against a stated arithmetic of "7 slots
  x 2 spellings x (1 or 4 displacers) x 2 depths = 68". The loop gives ONE
  displacer to six slots and four to `ApiDisplacedEndpoint`, so the real count is
  `(6 x 1 + 1 x 4) = 10` slot-displacer pairs `x 2 spellings x 2 depths = 40`.
  **No production change can move this number** — it is a pure function of the
  alphabet sizes in the test file. It was invisible before, because the property
  failed earlier at its per-case refusal assertion.
* **Fix:** corrected to the recounted 40, with the recount and the reason written
  into the constant's own doc. **No refusal assertion was touched and no alphabet
  was narrowed**; all 40 cases are refused with an empty walk. Raising the count
  instead by giving every slot all four displacers would emit duplicates, because
  `forge_slot_case` ignores the displacer for every slot but
  `ApiDisplacedEndpoint`, and a floor satisfied by duplicates is the dishonest
  counting this file's floors exist against.
* **This is a deviation from the plan's "every other file under `tests/` is
  byte-identical" verification line**, and it is reported rather than absorbed.
* **Commit:** `d458e69`.

### 3. [FINDING — reported, not laundered] A stale pre-fix handoff guard pinned a permit the rule must refuse

* **Found during:** Task 2.
* **Issue:** `the_marked_payload_splice_is_measured_pre_fix_and_deliberately_not_pinned_post_fix`
  asserts `answer.code == 0` for `rg "git status" {src,tests}`. Its **name**, its
  own comment ("This test asserts only the measurement"), its failure message
  ("if this changes **before `19-17` runs**") and `19-16-SUMMARY.md` ("measured
  exit 0 TODAY, **NOTHING asserted post-fix**") all say it constrains nothing
  after the rule lands. Mechanically it did. Plan 19-17's Task 2 item 5 orders
  this exact row measured after the rule exists and pinned with its clause, so
  the plan was written expecting the assertion `19-16` describes, not the one it
  wrote.
* **The fork, and why it was resolved this way.** Dropping `NestedPayload` from
  clause 2(a) would have made the row pass. That was rejected: it trades a
  DISCLOSED false positive for a possible false negative in a clause the plan
  states twice as load-bearing, and weakening a refusal to make a permit green is
  the exact direction five rounds of plan-check exist to prevent. **The rule was
  not weakened.** Editing an assertion about a PERMIT is the smaller loss than
  narrowing a refusal, and it is the one that leaves the control intact.
* **Fix:** the guard's contract was discharged — this plan confirmed it at exit 0
  against `09e83bd` before moving a production line, verbatim above — so its
  post-fix assertion was removed, the pre-fix measurement kept in the print and
  in a comment, and the post-fix verdict pinned in the NEW `#[test]` fn the plan
  ordered. The whole reasoning is written into the test file itself, not only
  here.
* **Commit:** `d458e69`.

**No assertion about a REFUSAL was edited or deleted anywhere.** The
`19-16` deviations above are both about rows that asserted a permit or counted
the generator's own cases.

## What remains uncovered

1. **`T-19-86`** — OPEN at `high`, by explicit user scoping decision. A governed
   program's own operand carrying a whole command line. All four rows
   re-measured at exit 0 against the built binary and the pin is unmodified and
   green.
2. **`T-19-91`** — OPEN at `high`. The record is corrected (a third arm, the bare
   `git push $REF` being cwd-dependent and permitted in the in-namespace
   configuration); the remedy is unchanged. `reflog` and `symbolic-ref` have **no
   second carrier** — git runs no `pre-push` and no `pre-commit` for either.
3. **`T-19-96`** — registered open, re-measured at exit 0 with its literal twin
   at exit 2 under `force_push_blocked`, **not fixed**. Widening the decision
   region to `classify_push`'s flags is the same move as closing `T-19-91`, and
   both are the next round's to decide.
4. **`T-19-74`** — accepted (AR-19-10); core rows re-measured permitted.
5. **`T-19-84`, `T-19-85`, and `T-19-61` … `T-19-73`** — open, unaccepted,
   untouched. `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not
   opened.
6. **`T-19-17r`** — the new over-refusal cost, accepted and pinned from both
   sides. Its widest row is `rg "git status" {src,tests}`, where nothing governed
   executes at all.
7. **The generative corpus cannot draw a splice inside an alternative's quoting**
   beyond the fail-closed trigger — `sh {-c,"git push --force origin main"}` is
   refused because a quote inside an alternative is unenumerable, not because the
   products were computed. That is correct and fail-closed, but it is a refusal
   the corpus does not distinguish from an enumerated one.

**`/gsd-secure-phase 19` is NOT cleared by this plan.** `T-19-86` and `T-19-91`
remain OPEN at `high` and `T-19-96` is registered open. Only the
**WRAPPER-OPERAND sub-class** of `T-19-60` is closed; `T-19-86` and `T-19-91` are
both sub-classes of it.

## Self-Check: PASSED

* `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-17-SUMMARY.md` — FOUND
* `src/envelope/policy.rs` — FOUND
* `src/envelope/hooks.rs` — FOUND
* `tests/envelope_literal_decision.rs` — FOUND
* `tests/envelope_wrapper_class.rs` — FOUND
* `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md` — FOUND
* `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` — FOUND
* commit `958b7a1` — FOUND
* commit `d458e69` — FOUND
* commit `2b0a901` — FOUND
