---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 21
subsystem: envelope-guard
tags: [security, callee-grammar, fail-closed, drift-pin, git-option-grammar]
status: complete
requires: [19-20]
provides:
  - "LeadingOptionGrammar — a three-valued grammar answer whose default is a refusal"
  - "GIT_GLOBAL_SELF_CONTAINED_OPTS — knowledge the guard never had"
  - "the real-git drift pin over all three option-grammar constants"
  - "T-19-102's one-entry fix with --signed as the control that proves it"
  - "the_guards_own_path_shells_out_to_nothing — a stricter local spawn control"
affects: []
tech-stack:
  added: []
  patterns:
    - "a hand-maintained enumeration of a CALLEE's grammar is pinned against the callee binary in a test"
    - "the absence of a required bit is an ANSWER, not a default"
    - "a file-level allowlist entry is compensated by a stricter control local to the file"
key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - tests/envelope_callee_grammar.rs
    - tests/spawn_seam_guard.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "Invert the failure direction rather than complete the list; the knowledge needed is one bit per option"
  - "The structural attached-value rules run FIRST, so the constants have less to know"
  - "The pin's source is the installed git binary, consulted at TEST time and never in the guard"
  - "PUSH_VALUE_OPTS gains recurse-submodules and nothing else; --signed stays refused"
  - "push_operands deliberately keeps its fail-open default; the drift pin covers it instead"
metrics:
  duration: "one session"
  completed: 2026-09-03
actuals:
  tokens: 118000
  tasks: 3
  commits: 4
requirements: [SAFE-02, SAFE-05, SAFE-06]
---

# Phase 19 Plan 21: The Rule for the Callee's Grammar Summary

**An unestablished verb slot is a refusal, not the next non-`-` word — the same
fail-closed treatment `resolve_program`'s wrapper axis has had since audit 3,
applied to the one arm of the same question that was never given it.**

---

## FIRST: `/gsd-secure-phase 19` is NOT cleared by this plan

- **`T-19-86` remains OPEN at `high`**, by explicit user scoping decision. Its
  four rows still exit 0 and its pin is green and unmodified.
- **`T-19-91` remains OPEN at `high`**, arms unweakened: `git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` at exit 0 with **no second
  carrier**, and bare `git push $REF` at exit 0 in the in-namespace
  configuration. No decision-operand rule was added and the denylist was not
  extended.
- **Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
  `T-19-91` are both sub-classes of it and both remain OPEN at `high`.
- `T-19-96`, `T-19-74`, `T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` are
  untouched and open.

---

## THE BLOCKER — one corpus row cannot be satisfied, and it was NOT edited

**`19-20`'s corpus carries two rows that no rule obeying this plan's
prohibitions can satisfy at once.**

| | Row | Pinned reason | Test |
|---|---|---|---|
| **A** | `git --super-prefix x status` | `envelope_assertion_failed` | `after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins` (RED at `097dba2`, must go green) |
| **B** | `git --super-prefix x push --force origin main` | `force_push_blocked` | `the_already_correct_planning_cells_keep_their_verdicts_as_controls` (green at `097dba2`, labelled "green today and after") |

**The proof.** The two commands carry **identical leading tokens** —
`--super-prefix x` — and `scan_leading` is a pure function of argv that sees
them identically up to the verb. Row A requires `scan_leading` to return
`Some(refusal)` for those tokens. `classify_git` returns that refusal
immediately, so row B is then `envelope_assertion_failed` too. Getting
`force_push_blocked` for row B requires the CLASSIFIER's verdict to take
precedence over the scan's refusal — **a filter in `classify_git` after
`scan_leading` returned, which is a second reading site and is explicitly
prohibited.** Per the plan's own instruction ("report it with the row, the
expected verdict and the measured one … never the assertion"), the row is left
RED.

**Row B's own comment shows it is a PRE-fix observation mis-labelled.** It reads
*"the two-sided twin of the stale-entry cell, which is what proves the entry is
what moves the verb index"* — it is `force_push_blocked` today precisely BECAUSE
`--super-prefix` is in `GIT_GLOBAL_VALUE_OPTS` and swallows `x`. Removing that
stale entry, which this round's own evidence requires, necessarily moves it.

**Safety is unaffected.** Both rows are REFUSED at exit 2 with an empty walk
under either rule; only the reason identifier differs, and
`envelope_assertion_failed` is the more honest of the two — naming
`force_push_blocked` for a command whose verb the guard admits it cannot
establish attributes the refusal to a mechanism that did not produce it.

Measured, verbatim:

```
`git --super-prefix x push --force origin main` must be refused UNDER
`force_push_blocked`. Asserting the identifier and not merely the exit code is
what stops a row from passing because it was refused for an unrelated cause
(D-24). Got: gsd-meta-manager envelope: REFUSED (reason: envelope_assertion_failed)
— this command carries the leading git option `--super-prefix`, whose grammar the
guard cannot establish …
```

The sibling row `git -C/tmp push --force origin main`, in the same test and the
same shape, WAS resolvable — see Deviations.

---

## `19-20`'s RED set, confirmed STILL RED before any production line moved

Run against `097dba2` with `rtk proxy cargo test --test <bin> --no-fail-fast`,
before a single `src/` byte was touched.

**`tests/envelope_callee_grammar.rs` — `test result: FAILED. 11 passed; 5 failed; 0 ignored`**

```
failures:
    after_19_21_a_leading_git_option_that_consumes_its_value_no_longer_hides_the_verb
    after_19_21_the_composition_rows_are_refused_which_proves_rounds_5_and_6_load_bearing
    after_19_21_the_in_namespace_push_with_a_value_taking_flag_is_no_longer_falsely_refused
    after_19_21_the_stale_and_bundled_planning_cells_are_refused
    after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins
```

```
---- after_19_21_a_leading_git_option_that_consumes_its_value_no_longer_hides_the_verb stdout ----
thread '...' panicked at tests/envelope_callee_grammar.rs:358:5:
assertion `left == right` failed: `git --attr-source HEAD push --force origin main`
must be REFUSED. It was measured at exit 0 against the BUILT BINARY at this file's
base commit, with a fresh envelope root and a walk after, and the verb git itself
reaches was confirmed with a two-sided probe of the real git binary.
stdout:  stderr:
Walked listing:
(the envelope root is empty)
  left: 0
 right: 2

---- after_19_21_the_in_namespace_push_with_a_value_taking_flag_is_no_longer_falsely_refused stdout ----
thread '...' panicked at tests/envelope_callee_grammar.rs:727:9:
assertion `left == right` failed: OVER-REFUSAL: this in-namespace push is FALSELY
REFUSED at exit 2 `push_outside_namespace` before the rule exists …
  command: git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w
  left: 2
 right: 0
```

**`tests/envelope_wrapper_class.rs` — `test result: FAILED. 32 passed; 2 failed; 0 ignored`**

```
failures:
    a_leading_option_whose_grammar_the_guard_does_not_know_is_not_a_verb
    an_option_the_installed_git_rejects_fails_closed_on_every_base
```

```
---- a_leading_option_whose_grammar_the_guard_does_not_know_is_not_a_verb stdout ----
thread '...' panicked at tests/envelope_wrapper_class.rs:5647:9:
A LEADING GIT OPTION CONSUMED THE VERB AND THE COMMAND WAS PERMITTED.
  command : "git --attr-source HEAD push --force origin main"
  base    : git push --force origin main
  splice  : ImmediatelyAfterTheProgram / ImmediatelyAfterTheProgram(--attr-source HEAD)
  got     : exit 0 reason permit(the guard answered nothing)
  seed    : 0x1912c0de5eed0060
  left: 0
 right: 2
```

**All seven were still RED. None came up green when red was expected.**

### Their transition

| # | Name | before | after |
|---|---|---|---|
| 1 | `after_19_21_a_leading_git_option_that_consumes_its_value_no_longer_hides_the_verb` | RED | **GREEN** |
| 2 | `after_19_21_the_in_namespace_push_with_a_value_taking_flag_is_no_longer_falsely_refused` | RED | **GREEN** |
| 3 | `after_19_21_the_stale_and_bundled_planning_cells_are_refused` | RED | **GREEN** |
| 4 | `after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins` | RED | **GREEN** |
| 5 | `after_19_21_the_composition_rows_are_refused_which_proves_rounds_5_and_6_load_bearing` | RED | **GREEN** |
| 6 | `a_leading_option_whose_grammar_the_guard_does_not_know_is_not_a_verb` | RED | **GREEN** |
| 7 | `an_option_the_installed_git_rejects_fails_closed_on_every_base` | RED | **GREEN** |

**All seven green.** `tests/envelope_wrapper_class.rs` is fully green at 34/34.

---

## The four commits, in order

| # | SHA | What |
|---|---|---|
| 1 | `e592f38` | `fix(19-21)`: an unestablished verb slot is a refusal, not the next non-`-` word |
| 2 | `fe49142` | `fix(19-21)`: `T-19-102` without a new mis-parse, and the three derived rows pinned |
| 3 | `077f5fe` | `test(19-21)`: the drift pin's git probe declared on the spawn allowlist, with a stricter local control |
| 4 | `58cd95e` | `docs(19-21)`: the plan-19-21 execution record, the three closures, and the blocker |

`git diff --stat HEAD~4..HEAD -- src/` is **exactly one file**,
`src/envelope/policy.rs` (788 insertions, 12 deletions). `hooks.rs` was not
opened. Neither `Cargo.toml` nor `Cargo.lock` is in the diff (`T-19-SC`).
`git diff --numstat` over `tests/` shows **ZERO deletions** across all commits
(86 insertions to `envelope_callee_grammar.rs`, 14 to `spawn_seam_guard.rs`).

---

## The gate arithmetic, STATED and CHECKED

Command: `rtk proxy cargo test --no-fail-fast`, counts read with
`rtk proxy grep 'test result:'` over a redirected log. A plain `cargo test`
fail-fasts at `driver_reattach` and `envelope_*` sorts after `driver_*`; a plain
`grep` reads a log the RTK hook has already stripped the `test result:` lines
from, which is how `19-19` first read 1364 instead of 1605 (D-34).

| | passed | failed | ignored | `passed + failed` |
|---|---|---|---|---|
| **Before** (`097dba2`) | 1623 | 8 | 13 | **1631** |
| **After** (`58cd95e`) | 1637 | 2 | 13 | **1639** |

```
new #[test] fns, counted with `git show <sha> -- <dir> | grep -cE '^\+\s*#\[test\]'`:
  e592f38  src/ = 4   tests/ = 0
  fe49142  src/ = 0   tests/ = 3
  077f5fe  src/ = 1   tests/ = 0
                          TOTAL NEW = 8

1631 + 8 = 1639   ==   the observed total.  The identity HOLDS exactly.
```

A red test **RAN**, so red→green leaves the total unchanged and replacing a body
adds none; every increase comes ONLY from new `#[test]` fns.

`cargo build` and `cargo clippy -- -D warnings` both exit 0. `cargo clippy
--tests` is NOT the gate — it already fails at base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs`, out of scope and untouched.

### The two failures

1. `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`
   (`tests/driver_reattach.rs`) — a **documented flake**, pre-existing,
   environmental, out of scope. Not fixed, not edited, not worked around. The
   other two documented flakes
   (`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
   and the `envelope_tracer` `ExecutableFileBusy` stub-write race carried in
   `deferred-items.md` since 19-07) did not fire; their absence is not a
   regression.
2. `the_already_correct_planning_cells_keep_their_verdicts_as_controls` — **the
   blocker above.** Not fixed, and not edited.

---

## Per-binary counts — all THIRTEEN `envelope_*` binaries RAN

| Binary | passed | failed | ignored | ran |
|---|---|---|---|---|
| `envelope_advisory` | 10 | 0 | 0 | ✔ |
| `envelope_argv_deletion` | 20 | 0 | 0 | ✔ |
| `envelope_callee_grammar` | 18 | **1** | 0 | ✔ |
| `envelope_command_position` | 18 | 0 | 0 | ✔ |
| `envelope_credential` | 6 | 0 | 0 | ✔ |
| `envelope_expansion_slots` | 32 | 0 | 0 | ✔ |
| `envelope_hook_refusals` | 7 | 0 | 0 | ✔ |
| `envelope_literal_decision` | 43 | 0 | 0 | ✔ |
| `envelope_pr_cap` | 11 | 0 | 0 | ✔ |
| `envelope_tracer` | 6 | 0 | 0 | ✔ |
| `envelope_wiring` | 14 | 0 | 0 | ✔ |
| `envelope_wrapper_bypass` | 13 | 0 | 0 | ✔ |
| `envelope_wrapper_class` | **34** | 0 | 0 | ✔ |

`envelope_argv_deletion`, `envelope_literal_decision`, `envelope_expansion_slots`
and `envelope_command_position` are FULLY GREEN and **byte-identical** — rounds
4, 5 and 6's evidence is untouched.

---

## Every `19-20` RED row, before and after, with its identifier and walk

All measured against `./target/debug/gsd-meta-manager` with a **fresh
`GSD_MM_ENVELOPE_ROOT` per row and the whole root walked afterwards**. Every
walk empty.

### The `T-19-100` rows — a live bypass closed

| Command | before | after |
|---|---|---|
| `git --attr-source HEAD push --force origin main` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD stash` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD update-ref -d refs/heads/main` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD config core.hooksPath /tmp/x` | 0 | 2 `hook_bypass_blocked` |
| `git --attr-source HEAD reflog delete HEAD@{0}` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD symbolic-ref HEAD refs/heads/x` | 0 | 2 `force_push_blocked` |
| `git --shallow-file /tmp/s push --force origin main` | 0 | 2 `force_push_blocked` |
| `git --shallow-file /tmp/s stash` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main` | 0 | 2 `hook_bypass_blocked` |
| `git --attr-source=HEAD push --force origin main` (CONTROL) | 2 | 2 `force_push_blocked` |

**Walk:** each of these rewrote a real ref before the fix — audit 6 confirmed
`git --attr-source HEAD push --force origin main` rewrote a bare remote's `main`.
Four have **no second carrier at all**: `stash`, `update-ref -d`,
`reflog delete` and `config core.hooksPath`, where disarming the hook IS the
loss of the carrier.

### The planning cells

| Command | before | after |
|---|---|---|
| `git --super-prefix push --force origin main` | 0 | 2 `envelope_assertion_failed` |
| `git -c a=b --attr-source HEAD push --force origin main` | 0 | 2 `force_push_blocked` |
| `git -pc user.name=x status` | 0 | 2 `envelope_assertion_failed` |
| `git --super-prefix x push --force origin main` | 2 `force_push_blocked` | **2 `envelope_assertion_failed` — THE BLOCKER** |
| `git -C/tmp push --force origin main` (control) | 2 `force_push_blocked` | 2 `force_push_blocked` |
| `git -- push --force origin main` (control) | 2 `force_push_blocked` | 2 `force_push_blocked` |
| `git -c core.hooksPath=/dev/null --attr-source HEAD push` (control) | 2 `hook_bypass_blocked` | 2 `hook_bypass_blocked` |
| `git - push --force origin main` (control) | 0 | **0** |

### The COST rows, each beside its permitted twin

| Command | before | after |
|---|---|---|
| `git --bogus-opt status` | 0 | 2 `envelope_assertion_failed` |
| `git --no-advice status` | 0 | 2 `envelope_assertion_failed` |
| `git --no-lazy-fetch status` | 0 | 2 `envelope_assertion_failed` |
| `git --super-prefix x status` | 0 | 2 `envelope_assertion_failed` |
| `git status` (twin) | 0 | **0** |
| `git log --oneline` (twin) | 0 | **0** |

### The COMPOSITION rows — rounds 5 and 6 load-bearing HERE

All five at exit 0 before, all at exit 2 `force_push_blocked` after:
`git >/dev/null --attr-source HEAD push …`,
`git --attr-source HEAD >/dev/null push …`, the
continuation-inside-the-option-name row, `bash -lc "…"`, and `echo hi && …`.
**The continuation row is the pin that turns red if round 6's deletion model is
ever removed**, whatever this round's rule does — the `\`+NL must be deleted
before `--attr-source` is even spelled.

### `T-19-102` and its controls

| Command | before | after |
|---|---|---|
| `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w` | 2 `push_outside_namespace` | **0** |
| `git push --signed no origin refs/heads/gsd-auto/alpha/w` (**CONTROL**) | 2 `push_outside_namespace` | 2 `push_outside_namespace` |
| `git push -o ci.skip origin refs/heads/gsd-auto/alpha/w` | 0 | 0 |
| `git push --push-option ci.skip origin refs/heads/gsd-auto/alpha/w` | 0 | 0 |
| `git push --recurse-submodules=on-demand origin refs/heads/gsd-auto/alpha/w` | 0 | 0 |
| `git push --recurse-submodules on-demand origin refs/heads/main` (out of namespace) | 2 | 2 |

---

## The three derived rows, with the clause that produced each

| Command | measured after | the clause |
|---|---|---|
| `git --attr-source $T push --force origin main` | 2 `force_push_blocked` | `--attr-source` now consumes `$T`, so `$T` is an option OPERAND outside the decision region — as `$MSG` is in `git commit -m "$MSG"`. Round 5's rule no longer reaches it; `classify_push`'s denied-flag arm answers. **The identifier MOVED while the exit code did not.** |
| `git --attr-source *.x push --force origin main` | 2 `force_push_blocked` | the same clause, through the glob half of round 5's rule rather than the parameter-expansion half |
| `git --attr-source HEAD {push,--force} origin main` | 2 `envelope_assertion_failed` | a **DIFFERENT** rule: `19-17`'s WHOLE-COMMAND brace rule in `resolve_program_with_head`, which is not a decision-word rule and does not care which slot the braces occupy. The one of the three whose identifier did NOT move, and the row that shows this round did not pay for its rule by narrowing an earlier one's |

**None became PERMITTED**, so the finding those rows were watched for — a value
slot that stops being guarded, over-consumption in the `--super-prefix`
direction — did not arise. Each is its own new `#[test]` fn; no existing body
was replaced.

---

## The design question, answered — four options, three rejected

| # | Option | Verdict and cost |
|---|---|---|
| i | Complete `GIT_GLOBAL_VALUE_OPTS` | **REJECTED.** Closes today's two cells and is wrong again at the next git release; the list was already wrong in BOTH directions against the installed git. Completing an enumeration does not change the failure DIRECTION, which is the defect |
| ii | Ask git for its grammar at GUARD time | **REJECTED on two independent grounds.** The guard runs synchronously on the agent's `PreToolUse` critical path and `push_needs_resolved_dests` exists because a reproduced 180–240 s hang made per-call shelling out unacceptable; and a guard that asks the program it guards to describe its own grammar can be lied to by a `git` earlier on `PATH`. There is also nothing machine-readable to read — git's globals live in prose, and `--list-cmds=` lists commands, not options |
| iii | Derive at build or envelope-construction time | **REJECTED.** The probing binary is not the guarded binary, the result is non-hermetic, and a probe that failed would have to fail closed — a guard nobody can build |
| iv | **ADOPTED** — invert the failure direction, pin the constants against real git in a TEST | **The knowledge required is one bit per option: does it consume the next word.** Three structural rules supply it for free and run FIRST — a `--`-prefixed token containing `=` is self-contained whatever the option is, `--` ends the options, a non-`-` token ends the scan — so only the remaining spellings need a constant, and the ABSENCE of the bit becomes a refusal |

---

## The drift pin — its limit, its UNPROBED set, and what it catches today

Four new `#[test]` fns in `policy.rs`'s own `#[cfg(test)] mod tests` run the
two-sided probe (`git <opt> version` vs `git <opt> XVALUE version`) against the
installed `git` over **every entry of both leading-option constants and of
`PUSH_VALUE_OPTS`**. It exists because `GIT_GLOBAL_VALUE_OPTS` had **no pin and
no test reference anywhere** — audit 6 measured exactly two mentions of it in
the whole repository.

**It catches `--super-prefix` today.** Verified by MUTATION, not by argument:
re-adding a rejected option (`--no-advice`) to the value-taking constant, and
adding `--attr-source` to the self-contained one, turned all three
leading-option pins red; adding `signed` to `PUSH_VALUE_OPTS` turned the push
pin red. All mutations reverted.

- **Negative controls in both arms**: `--bogus-opt` must classify as NOT
  ACCEPTED, and a value-taking spelling must FAIL the self-contained probe.
- The two constants are asserted **disjoint**.
- **UNPROBED, bounded at two and named: `--help` and `-h`.**
  `git --help XVALUE version` answers `No manual entry for gitXVALUE`, which
  neither reaches a verb nor names the following word as a value, so neither arm
  of a probe of this shape fires. They are in **neither** constant and take the
  fail-closed path.
- Floors with their arithmetic beside them: **≥ 6** value-taking (the six the
  corpus's class-1 alphabet splices by name), **≥ 8** self-contained (the seven
  pinned by name elsewhere, plus one). Every floor's failure message says the
  correct response is to RESTORE entries, never to lower the floor.
- It does **NOT** skip when git is absent; it panics. A skipped pin is a
  fail-open pin.

**Its limit, stated correctly rather than comfortably.** It pins the constants
against the **DEVELOPER's** git, not the runtime git, and **the fail-closed
default covers only the SILENT case.** A MISSING bit costs a refusal. A **WRONG**
bit costs a **shifted verb** — a self-contained entry a runtime git treats as
value-taking, or a value-taking entry it rejects, makes `scan_leading` step over
or land short of the real verb, which is exactly the `--super-prefix` mechanism
this round measured at exit 0. **So this pin is the only control over the
wrong-bit direction, and a constant that outruns the runtime git is a bypass
rather than an over-refusal.**

---

## The over-refusal cost, measured from both sides

**ZERO on git 2.43.0.** Every option this git accepts is classified by the probe
and enumerated, so the only rows moving permitted → refused are ones git ITSELF
rejects — `git --bogus-opt status`, `git --super-prefix x status`,
`git -pc user.name=x status` — refusals of commands that already do nothing.

**One refusal per newly added global option on a FUTURE git**, until the
constant learns it. `--no-advice` and `--no-lazy-fetch` are the measured
stand-ins: real global options in later releases, rejected by this one, both in
the corpus so the future cost is checkable rather than argued.

**The three-step recovery, in the order the refusal message offers it:**

1. Spell the option with its value attached (`--option=value`) — needs no
   constant change, because git's own grammar makes an attached value
   self-contained. **Offered first and never alone**, because an attached
   spelling is always self-contained but is **not always accepted**:
   `git --shallow-file=/tmp/s version` answers `unknown option:
   --shallow-file=/tmp/s` on this git, while `--attr-source=`, `--git-dir=`,
   `--namespace=` and `--work-tree=` all reach the verb.
2. Drop the option.
3. Add the spelling to the constant, which the drift pin will name.

The message names the **OPTION TOKEN** and never quotes the command back
(SAFE-04), on the same footing as the existing `git -c {assignment}` refusals.

---

## The controls that show this is a grammar MODEL, not a blanket refusal

- **Twelve ordinary invocations still at exit 0**: `git --no-pager status`,
  `git --no-pager log --oneline`, `git -c user.name="$NAME" commit -m x`,
  `git --git-dir=/tmp/g status`, `git -C /tmp status`, `git --bare status`,
  `git --literal-pathspecs status`, `git --no-optional-locks status`,
  `git --exec-path status`, `git --version`, `git --attr-source HEAD status`,
  `git --shallow-file /tmp/s log --oneline`. **`git --no-pager status` is this
  axis's `ls {git,svn}-repo`.**
- `git - push --force origin main` still at exit 0 — the rule was not widened
  into non-`-`-prefixed words — and `git -- push --force origin main` still at
  exit 2 `force_push_blocked`.
- **The four mechanism pins, GREEN and UNMODIFIED**, so rounds 5 and 6 did not
  become dead code:
  - `Token.literal` is `false` for `pus?` and `true` for `--attr-source`, `HEAD`
    and `push` — round 5's bit is RIGHT about every word of the bypass line and
    was **not falsified to obtain a refusal**.
  - `git >/dev/null push --force origin main` refused and
    `git x2>/tmp/o push --force origin main` **PERMITTED** — round 6's deletion
    model is non-dead.
  - Rule B reports `head_is_command_position == false` for the severed spellings
    and `true` for `{ git status; }`.
  - `policy::is_separator(">") == false`, `SEPARATORS` **byte-identical**, and
    `separators_are_named_in_one_list_that_the_predicate_reads` green.
- `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
  green under `19-20`'s absolute byte floors, and **no wrapper name is
  load-bearing**. It FIRED once mid-round and correctly — see Deviations.

---

## The decision region did NOT move, and no second reading site was built

Checked mechanically over `HEAD~4..HEAD`:

- `git diff --stat -- src/` → **one file**, `src/envelope/policy.rs`.
- `hooks.rs` → **zero** files in the diff.
- No new `ParkReason` variant; the only variant named in added lines is
  `EnvelopeAssertionFailed`.
- No hunk touches `first_unreadable_decision_word`, `config_key_operand_index`,
  `subcommand_word_indices`, `scan_gh_api` or `resolve_program_with_head`; the
  post-filter gains no arm and no wildcard.
- The guard's own path spawns nothing — asserted mechanically by the new
  `the_guards_own_path_shells_out_to_nothing`, which cuts this file at its first
  `#[cfg(test)]` and finds no spawn marker in the production half.

---

## Deviations from Plan

### 1. [Rule 3 — blocking] `leading_git_option` gained a fourth arm the plan's list does not carry

- **Found during:** Task 1, immediately after the first measurement.
- **Issue:** With exactly the arms the plan enumerates, `git -C/tmp push --force
  origin main` moved from `force_push_blocked` to `envelope_assertion_failed`,
  turning `the_already_correct_planning_cells_keep_their_verdicts_as_controls`
  red — a file this round may not edit.
- **Fix:** a fourth arm — a single-dash token longer than two characters whose
  first two characters name a member of `GIT_GLOBAL_VALUE_OPTS` carries that
  option's value ATTACHED, so it is self-contained. **The restriction to
  value-taking heads is what makes it a grammar claim rather than a
  convenience**: only an option that takes a value can have one attached, and a
  self-contained option followed by more characters is a BUNDLE — git 2.43.0
  accepts no short-option bundling at all. `-pc user.name=x`, whose head `-p` is
  self-contained, therefore does **not** match and stays unestablished, which is
  its pinned verdict.
- **Files:** `src/envelope/policy.rs` · **Commit:** `e592f38`

### 2. [Rule 3 — blocking] `tests/spawn_seam_guard.rs`, outside `files_modified`

- **Found during:** the first full `--no-fail-fast` gate run.
- **Issue:** `every_process_spawn_site_in_src_is_on_the_allowlist` flagged
  `src/envelope/policy.rs`: the drift pin's probe uses `std::process::Command`,
  and that control's line filter strips comments but not `#[cfg(test)]` modules.
- **Fix:** the allowlist entry, which the control's own doc and failure message
  say must be added **in the same commit** as the spawn site, and for which
  three entries of exactly this shape already exist
  (`src/executor/outcome.rs`, `src/driver/kill.rs`, `src/driver/liveness.rs`).
  Because a file-level entry makes that control stop looking at the file
  entirely, the property was **re-asserted at a stricter granularity** by a new
  in-file test that cuts `policy.rs` at its first `#[cfg(test)]` and asserts no
  spawn marker in the production half, with positive controls so the absence
  cannot pass over a truncated string.
- **Files:** `tests/spawn_seam_guard.rs`, `src/envelope/policy.rs` ·
  **Commit:** `077f5fe`

### 3. [Recorded] The anti-vacuity control fired mid-round, and it was right

`GIT_GLOBAL_UNPROBED_OPTS` was first written as a `#[cfg(test)]` constant beside
`GIT_GLOBAL_VALUE_OPTS`, at line ~607. `tests/envelope_wrapper_class.rs`'s
`production_code` stripper treats the **first** line reading `#[cfg(test)]` as
the end of production logic, so its view of `policy.rs` was truncated at line
607 and `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
correctly reported that its positive control had failed. The constant was moved
into the test module and the reason is recorded in its own doc. **Nothing in
that control was weakened**; the byte floors `19-20` set are untouched.

No auto-fixes under Rule 1 or 2 were required beyond the above. No Rule 4
architectural decision arose. No authentication gates occurred.

---

## Known Stubs

**None.** Every row is driven against the built binary and asserted, or driven
and printed with the reason it is not asserted.

---

## `T-19-17r` — the bookkeeping gap, still OUTSTANDING

`19-17-SUMMARY.md` calls it "accepted". Audits 5 and 6 both confirmed the
measurement and both pins (`tests/envelope_literal_decision.rs:1355-1379`) and
**both explicitly declined to make the acceptance, because accepting a risk is a
human decision.**

**This plan did NOT accept it.** No `AR-19-13` row and no Accepted-Risks-Log row
was added — the row count for that risk ID is **0**, verified after all four
commits — and no register row was added. The gap is recorded as **OUTSTANDING**
for the third round running. The next round either adds the log row or drops the
word.

---

## The `glab --host` forge cell — carried forward, callee UNCONFIRMED, unfixed

`glab --host gitlab.com mr create --title x` → exit 0 with **ZERO** ledger
lines; `glab --hostname …` → exit 0 with one. `FORGE_VALUE_OPTS` is the same
hand-maintained enumeration of a callee's option grammar in a third component,
failing in the **under-counting** direction (SAFE-06).

**`glab` is not installed on this machine**, so whether glab accepts `--host` as
a separate-value global flag is **not confirmed against the callee**, and **this
is not claimed as a live bypass.** `FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and
`subcommand_word_indices` were **not touched**. It needs a machine with `glab`
installed.

---

## What remains uncovered

1. **`T-19-86` — OPEN at `high`, by explicit user scoping decision.** Its four
   rows still exit 0 and must: `git submodule foreach git push --force origin
   main`, `git rebase -x "git push --force origin main" HEAD~3`,
   `git bisect run sh -c "git push --force origin main"`,
   `git -c alias.p='!git push --force origin main' p`. Because it remains open
   at `high`, **this plan does not clear `/gsd-secure-phase 19`**.
2. **`T-19-91` — OPEN at `high`, arms unweakened, no remedy added.**
   `git reflog $S`, `git reflog show $S` and `git symbolic-ref $S` at exit 0
   with **no second carrier**; bare `git push $REF` at exit 0 in-namespace.
3. **The blocker row** — `git --super-prefix x push --force origin main` pinned
   at `force_push_blocked` in a file this round may not edit, incompatible with
   `git --super-prefix x status` pinned at `envelope_assertion_failed`. Both are
   refused at exit 2 under either rule; only the identifier differs.
4. **`T-19-96`** — a glob in a PUSH FLAG, one slot outside the decision region.
   Registered by `19-16`, not fixed.
5. **`T-19-74`** — core rows frozen and re-measured permitted.
6. **`T-19-84`, `T-19-85`, and `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision. Untouched.
7. **The `glab --host` forge cell** — registered, callee unconfirmed, out of
   scope.
8. **`T-19-17r`** — bookkeeping gap OUTSTANDING; the acceptance deliberately
   unmade.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` are both sub-classes of it and both remain OPEN at `high`, so
**this plan does not clear `/gsd-secure-phase 19`.**

---

## Self-Check: PASSED

- `src/envelope/policy.rs` — FOUND (modified)
- `tests/envelope_callee_grammar.rs` — FOUND (86 insertions, 0 deletions)
- `tests/spawn_seam_guard.rs` — FOUND (14 insertions, 0 deletions)
- `19-SECURITY.md`, `deferred-items.md` — FOUND (appended)
- Commits `e592f38`, `fe49142`, `077f5fe`, `58cd95e` — all FOUND in `git log`
- `src/` files across all four commits: **1**
- `tests/` deletions across all four commits: **0**
- `Cargo.toml` / `Cargo.lock` in the diff: **none**
- Accepted-Risks-Log rows for the `T-19-17r` risk ID: **0**
- New `ParkReason` variants: **0**
- `passed + failed` = **1639** = 1631 + 8 new `#[test]` fns
