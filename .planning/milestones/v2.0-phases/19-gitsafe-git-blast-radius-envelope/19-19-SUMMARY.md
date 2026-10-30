---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 19
subsystem: envelope
status: complete
tags: [security, shell-parsing, tokenizer, redirection, line-continuation, T-19-97, T-19-98, T-19-99]
requires:
  - 19-18 (the deletion-axis corpus, observed RED with zero `src/` hunks)
  - 19-17 (the inverted literalness rule and the exhaustive post-filter)
provides:
  - "`tokenize` models bash's redirection production — operator and target deleted, no token for either"
  - "`\\`+newline consumed as a line continuation producing no character, in the unquoted arm and the double-quote loop"
  - "`Segment::redirection_unresolvable` — the fail-closed residue, refused in the arms `resolve_program_with_head` already had"
affects:
  - src/envelope/policy.rs
tech-stack:
  added: []
  patterns:
    - "deletion modelled inside the ONE walk, so a deleted word never becomes a `Token` and no second reading site is needed"
    - "a finite grammar production plus a fail-closed residue, rather than an enumeration"
key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - tests/envelope_argv_deletion.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "Design option (a) — `tokenize` models deletion — taken; option (b), refusing any governed command carrying a shell-deleted token, rejected on the measured cost of turning a COUNTED `gh pr create … > /tmp/o` into a false positive"
  - "A `{name}` fd-allocation prefix is deliberately NOT modelled; it is marked unresolvable and refuses, disclosed with its permitted-half twin"
  - "An IO_NUMBER is a BARE digits-only run tracked while the word is consumed, not a test over the dequoted word text — measured against bash"
  - "`Token.literal` deliberately left TRUE for redirections and continuations: a deletion is not a rewrite"
metrics:
  duration: ~2h
  completed: 2026-08-29
actuals:
  tokens: 61000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 19: The Deletion Axis — Modelling What The Shell Removes From argv

Bash deletes a redirection's operator and target, and a `\`+newline's two
characters, before `execve`; `tokenize` now deletes them too, so the argv the
classifier reads is the argv that runs.

## FIRST: `/gsd-secure-phase 19` is NOT cleared

* **`T-19-86` — OPEN at `high`**, by explicit user scoping decision. Its four
  rows still exit 0 and its control is green and unmodified.
* **`T-19-91` — OPEN at `high`**, three arms: `reflog $S`, `reflog show $S` and
  `symbolic-ref $S` at exit 0 **with no second carrier**, plus the bare
  `git push $REF`, which is cwd-dependent and permitted in the in-namespace
  configuration — a third arm, **not** "already fails closed". Record
  unweakened.
* **`T-19-96`** registered open; **`T-19-74`** core rows frozen at their
  measured verdicts; **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** open,
  unaccepted and untouched.
* **The `T-19-17r` bookkeeping gap stays OUTSTANDING.** This plan did **not**
  accept it, added **no `AR-19-13` row**, and did not write the word "accepted"
  about it — that is a human decision audit 5 explicitly declined to make.
* Only the **wrapper-operand** sub-class of `T-19-60` is closed. `T-19-86` and
  `T-19-91` are both sub-classes of it and both remain open.

## The RED confirmation — verbatim, before any production line moved

Run against `1d1229e` (19-18's post-state), `rtk proxy cargo test --no-fail-fast`:

```
passed=1600 failed=5 ignored=13 TOTAL=1605
```

Exactly the total `19-18-SUMMARY.md` records. The complete failing set, all five
of the names 19-18 handed forward:

```
after_19_19_a_line_continuation_inside_or_beside_a_decision_word_is_refused
after_19_19_a_redirection_between_the_program_and_its_decision_words_is_refused
after_19_19_the_forge_rows_are_counted_rather_than_bypassed_uncounted
after_19_19_the_planning_cells_are_refused_because_they_are_the_same_class
a_word_the_shell_deletes_between_the_program_and_its_decision_words_is_not_a_decision_word
```

Per-binary, verbatim from the same run:

```
envelope_argv_deletion    FAILED. 13 passed; 4 failed; 0 ignored
envelope_wrapper_class    FAILED. 28 passed; 1 failed; 0 ignored
```

Two representative panic messages:

```
`git >/dev/null push --force origin main` must be REFUSED. It was measured at exit 0
against the BUILT BINARY at this file's base commit, with a fresh envelope root and a
walk after, and confirmed under a bash shim to assemble the dangerous command.
Walked listing: (the envelope root is empty)      left: 0   right: 2

command : "git >/dev/null push --force origin main"
base    : git push --force origin main
slot    : 1
splice  : AsItsOwnWord(>/dev/null)
got     : exit 0 reason permit(the guard answered nothing)
seed    : 0x1912c0de5eed0060
```

**A methodology note that nearly corrupted this number.** My first attempt to
read the totals piped `cargo`'s output through a plain `grep`, which the RTK
hook rewrote — and RTK strips exactly the `test result:` lines a count is read
from (D-34). That produced **1364**, not 1605, and the discrepancy is what
surfaced the mistake. Every number in this SUMMARY comes from `rtk proxy grep`
over an `rtk proxy cargo test --no-fail-fast` log.

**All five are GREEN after.** The transition is named per-suite below.

## The three commits, in order

| # | sha | what |
|---|---|---|
| 1 | `11745ca` | `feat(19-19): tokenize models the deletion bash performs` |
| 2 | `b7f9684` | `fix(19-19): refuse an unresolvable deletion in the arms that already exist` |
| 3 | `7b0bcbe` | `docs(19-19): the execution record and the round-6 closures` |

## The gate, with the arithmetic stated and CHECKED

Command: `rtk proxy cargo test --no-fail-fast`.

| | passed | failed | ignored | `passed + failed` |
|---|---|---|---|---|
| Before (`19-18`'s post-state, re-measured here) | 1600 | 5 | 13 | **1605** |
| After (this plan) | 1610 | 0 | 13 | **1610** |

**A red test RAN**, so red→green leaves the total unchanged and the only source
of increase is NEW `#[test]` fns. Counted from `git show`: `git diff 1d1229e..HEAD`
shows **seven** added `fn …()` lines, of which **two are the RENAMED bodies of
the two pins 19-18 named** — a replacement adds no test. So **5 new `#[test]`
fns**:

* `policy.rs` test module — `the_redirection_and_continuation_table_recovers_exactly_the_argv_bash_runs`,
  `an_unenumerable_brace_word_is_distinguishable_from_an_enumerated_governed_one`
* `tests/envelope_argv_deletion.rs` — `the_expansion_carrying_redirections_stay_refused_under_a_changed_reason_identifier`,
  `the_fd_allocation_prefix_is_refused_by_the_unresolvable_clause_and_not_by_its_siblings`,
  `an_unresolvable_redirection_refuses_a_governed_program_and_spares_an_ungoverned_one`

```
1605 + 5 = 1610   =   observed passed + failed        ✓ identity holds exactly
```

**Failures: 0.** The two flaky `driver_reattach` tests passed in this run, as
they did in 19-18's; that is not a change this plan made and not a fix.

### Per-binary counts, all six envelope suites (and the rest)

| suite | before | after |
|---|---|---|
| `envelope_argv_deletion` | 13 passed **4 failed** | **20 passed 0 failed** |
| `envelope_wrapper_class` | 28 passed **1 failed** | **29 passed 0 failed** |
| `envelope_literal_decision` | 43 passed 0 failed | 43 passed 0 failed |
| `envelope_expansion_slots` | 32 passed 0 failed | 32 passed 0 failed |
| `envelope_command_position` | 18 passed 0 failed | 18 passed 0 failed |
| `envelope_wrapper_bypass` | 13 passed 0 failed | 13 passed 0 failed |
| `envelope_advisory` 10 · `envelope_credential` 6 · `envelope_hook_refusals` 7 · `envelope_pr_cap` 11 · `envelope_tracer` 6 · `envelope_wiring` 14 | all 0 failed | all 0 failed |

`cargo build` exits 0; `cargo clippy -- -D warnings` exits 0; `cargo clippy
--all-targets` reports **exactly the 4 documented pre-existing warnings**, all
at `src/browser.rs:155-157` and `src/project_creator.rs:146` — none introduced
here.

## The design question, answered explicitly

**Chosen: (a) — `tokenize` MODELS deletion**, so decision-word indices are
computed over the surviving argv. **Rejected: (b)** — refusing any governed
simple command containing a token the shell deletes. Option (b)'s cost, all
permitted today and all refused under (b):

```
exit 0   git log > out                     exit 0   git commit -m "x" >> build.log
exit 0   git status > /tmp/s.txt           exit 0   gh pr list 2>/dev/null
exit 0   git diff > /tmp/d.patch           exit 0   git fetch origin 2>&1 | tee log
exit 0 + ONE LEDGER LINE   gh pr create --title x > /tmp/o
```

That last row is decisive: (b) turns a correctly **COUNTED** pull-request
creation into a refusal — the exact trade `T-19-93`'s COUNT bar forbids.

## The production as implemented

```
[ IO_NUMBER ] OPERATOR WORD       operator AND target both DELETED, no token for either
OPERATOR ∈ { < > >> <> >| <& >& &> &>> << <<- <<< }
IO_NUMBER = a BARE digits-only run since the start of the word, and nothing else
```

* `&>` / `&>>` recognised by an **earlier match arm**, before `&` reaches the
  separator arm and its `&&` two-character consumption.
* Any unquoted `<`/`>` the production cannot complete marks the simple command
  **UNRESOLVABLE** and refuses through the arms that already existed.
* `\`+newline consumed producing **no character**, in the unquoted arm and the
  double-quote loop, **never starting a word**. The single-quote loop is
  untouched; audit 5's three discarded rows were not re-added.
* **`literal` NOT cleared** anywhere — a deletion is not a rewrite.

**`SEPARATORS` is BYTE-IDENTICAL** — `git diff HEAD~N` reports **0** changed
lines matching `SEPARATORS: &[` — and **`is_separator(">")` is still false**,
with `separators_are_named_in_one_list_that_the_predicate_reads` green and
unmodified. Only the doc's second sentence's unexamined consequence was
corrected.

## Every `19-18` RED row: before → after, with reason identifier and derivation

All 25 non-forge rows moved from **exit 0** to **exit 2**. Derivation for each:
the redirection or the continuation is deleted, and the surviving argv is
exactly the argv `the_unwrapped_refusals_still_fire…` measures at exit 2.

| row | after | derivation |
|---|---|---|
| `git >/dev/null push --force origin main` | `force_push_blocked` | operator+target deleted |
| `git 2>/dev/null push …`, `git 1>/dev/null push …` | `force_push_blocked` | bare digits = IO_NUMBER, discarded |
| `git >>/tmp/x push …` | `force_push_blocked` | `>>` matched before `>` |
| `git > /tmp/o push …` | `force_push_blocked` | whitespace allowed before target |
| `git <<<x push …` | `force_push_blocked` | `<<<` matched before `<<`, `<` |
| `git push>/dev/null --force …` | `force_push_blocked` | attached operator terminates the word; `push` survives |
| `bash -lc "git >/dev/null push …"` | `force_push_blocked` | `NestedPayload` re-split carries the fact |
| `echo hi && git >/dev/null push …` | `force_push_blocked` | per-command reset at the real operator |
| `git >/dev/null stash` | `force_push_blocked` | **no second carrier** |
| `git >/dev/null update-ref -d refs/heads/main` | `force_push_blocked` | **no second carrier** |
| `git >/dev/null config core.hooksPath /tmp/x` | `hook_bypass_blocked` | **no second carrier** |
| `git >/dev/null -c core.hooksPath=/dev/null push …` | `hook_bypass_blocked` | step 1's identifier kept (D-24) |
| `git pu\<NL>sh --force …` | `force_push_blocked` | continuation deleted, word is `push` |
| `git \<NL>push --force …` | `force_push_blocked` | continuation does not START a word |
| `git "pu\<NL>sh" --force …` | `force_push_blocked` | the double-quote loop's own branch |
| `bash -lc "git pu\<NL>sh --force …"` | `force_push_blocked` | nested payload |
| `git sta\<NL>sh` | `force_push_blocked` | **no second carrier** |
| `git config core.hooks\<NL>Path /tmp/x` | `hook_bypass_blocked` | **no second carrier** |
| `git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force …` | `hook_bypass_blocked` | both words continued |
| `git &>/tmp/o push …` | `force_push_blocked` | `&>` before the separator arm |
| `git >\|/tmp/o push …`, `git <>/tmp/o push …` | `force_push_blocked` | two-character operators |
| `git <<EOF push …` | `force_push_blocked` | heredoc delimiter is the target |
| `touch input.txt && git <input.txt push …` | `force_push_blocked` | precondition row |

### The four FORGE rows — a restored COUNT, evidenced by a WALKED LEDGER LISTING

Not by an exit code. The bypass was **uncounted, not exceeded**, and the cap has
no second carrier (`T-19-35`).

| row | before | after |
|---|---|---|
| `gh >/dev/null pr create --title x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |
| `gh >/dev/null api repos/o/r/pulls -f title=x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |
| `glab >/dev/null mr create --title x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |
| `gh p\<NL>r create --title x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |

Derivation: the subcommand words are back in their slots, so `pr_command_label`
matches and the creation is COUNTED. Refusing them would have traded a restored
count for a false positive on ordinary forge syntax.

### The three rows 19-18 could not derive — measured, and pinned as ADDED tests

| row | before | after | clause |
|---|---|---|---|
| `git >$F push --force origin main` | 2 `envelope_assertion_failed` | 2 `force_push_blocked` | **identifier change, not a verdict change** — the target is deleted whether or not its text is knowable, so the bit never sees it |
| `git >*.log push --force origin main` | 2 `envelope_assertion_failed` | 2 `force_push_blocked` | same |
| `git {v}>/tmp/o push --force origin main` | exit 0 | 2 `envelope_assertion_failed` | **a different clause from its six siblings** — the `{name}` fd prefix is not modelled |

## The two over-refusals REMOVED — cost measured in the direction nobody expected

| row | before | after | twin | derivation (from the measured message) |
|---|---|---|---|---|
| `git push origin refs/heads/gsd-auto/alpha/w > log.txt` | **exit 2 `push_outside_namespace`** | **exit 0** | one-line form, exit 0 | the redirection word was read as an **EXTRA REFSPEC** — *the refspec `>` resolves to `refs/heads/>`* |
| `git push \<NL> origin refs/heads/gsd-auto/alpha/w` | **exit 2 `push_outside_namespace`** | **exit 0** | one-line form, exit 0 | the whitespace after the continuation **FLUSHED it into its own WORD in the REMOTE slot**, displacing every operand one slot right — *the refspec `origin` resolves to `refs/heads/origin`* |

That is `T-19-97`'s displacement arriving through `T-19-98`'s mechanism, which
is why one rule moves both.

**Replacing those two `#[test]` bodies was this plan's ONLY test deletion.**
Verified by reading the diff **HUNKS**, not the numstat total:

```
git diff HEAD~1 -- tests/   →   1 file changed, 201 insertions(+), 50 deletions(-)
```

and **every one of the 50 deleted lines lies inside exactly those two fns**. No
other file under `tests/` was touched — `envelope_wrapper_class.rs`,
`envelope_literal_decision.rs`, `envelope_expansion_slots.rs` and
`envelope_command_position.rs` are byte-identical.

Each replaced pin keeps its one-line twin **and gains an out-of-namespace
control** (`git push origin refs/heads/main > log.txt`, still refused under
`push_outside_namespace`), so what the rule removed is the redirection being
read as a decision word — not the refspec check.

## The controls proving this is a deletion MODEL, not a blanket refusal

* **Over-deletion control still PERMITTED**: `git x2>/tmp/o push --force origin
  main`. Bash gives `ARGV[git]: [x2] [push] [--force] [origin] [main]`.
* **Quoted `>` rows still permitted**: `git commit -m ">"`, `git log
  --grep='>'`, `git commit -m "a > b"`, `rg ">" src/`, `--push-option="a>b"`.
* **Ordinary-redirection corpus still permitted**, and `gh pr create --title x >
  /tmp/o` **still COUNTED** with one ledger line.

## What the rule NEWLY REFUSES, each beside its permitted twin

| newly refused | permitted twin | clause |
|---|---|---|
| `git >` | `ls >`, `cargo test >` | unresolvable; the mark refuses only when a GOVERNED program is reached. **Bash does not run `git >` either** (`syntax error near unexpected token 'newline'`) |
| `git {v}>/tmp/o push --force origin main` | `git >/dev/null push --force origin main` (modelled) | `{name}` fd allocation not modelled |
| `git {v}>/tmp/o status` (was exit 0) | `git >/dev/null status` (exit 0) | same clause — the mark asks whether argv is KNOWABLE, not whether it is dangerous |

**Net: two shapes newly refused, two measured false refusals removed.**

## The decision region did not move, and no second reading site was needed

`first_unreadable_decision_word` is **unchanged**, including its single
`at(index, role)` closure — confirmed by reading and by the diff's hunk headers,
which touch neither it nor `scan_leading`, `config_key_operand_index`,
`subcommand_word_indices` or `scan_gh_api`. A deleted word never becomes a
`Token`, so every decision index is over the surviving argv automatically.
Round 3's principle is **discharged, not weakened**.

`resolve_program_with_head` keeps one arm per `ProgramResolution` variant with
**no wildcard** (verified by grep); there is still exactly one post-filter.

## Audit 5's disclosed corpus limit — ADDRESSED

`an_unenumerable_brace_word_is_distinguishable_from_an_enumerated_governed_one`,
in `policy.rs`'s own `#[cfg(test)] mod tests`, pins `products == None` for
`{-c,"git push --force …"}` (a quote inside an alternative — UNENUMERABLE) and
`products == Some(["git"])` for `{g..g}it` (ENUMERATED, governed by PRODUCT),
with `{git,svn}-repo` as the control keeping the `true` answer non-vacuous.
It had to be a unit test because both refusals reach the same
`envelope_assertion_failed` identifier at the guard boundary — which is exactly
why the corpus could not tell them apart.

## Deviations from plan

**1. [Rule 1 — Bug found while implementing] A quoted digit run is not an
IO_NUMBER.** The plan specified the fd prefix as "a DIGITS-ONLY run since the
start of the word". My first implementation tested that over the word's
recovered `text` — whose quoting has already been removed. Measured against
bash: `git "2">/tmp/o push --force origin main` gives `ARGV[git]: [2] [push]
[--force] [origin] [main]`, so a **quoted** digit run is a real argv word.
Reading `text` would have **over-deleted** it — the exact failure direction this
axis's control exists to catch. Fixed by tracking bare-ness while the word is
consumed (the same discipline `Token::literal` is built on), and pinned as a row
of the tokenizer table. Found by measurement, not by the plan.

**2. [Not a deviation, but a plan expectation that did not materialise]
`src/envelope/hooks.rs` needed NO change.** The plan listed it in
`files_modified` and asked for the fact to be threaded from `guard_in`'s split
and the `NestedPayload` re-split. Both already go through
`split_segments_with_heads` and `resolve_program_with_head`, so the fact threads
by construction. This is the same property that made the second reading site
unnecessary, so it is the plan's own principle holding rather than a shortfall.
No file outside `policy.rs` changed in `src/`.

**3. [Rule 3 — Blocking issue] A pre-existing control tripped on documentation
volume, and was NOT edited.** `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
carries a positive control requiring that stripping comments and the
`#[cfg(test)]` module leave **more than a quarter** of `policy.rs` — otherwise
an absence assertion over what remains proves little. This round's doc comments
and two new test fns pushed it to **24.88%** (65 947 of 265 008 bytes) and
turned it red. **The assertion was not touched**; redundant prose was tightened
until production code was back above the floor. Reported below as a finding.

## Findings — recorded, not fixed

**The production/comment ratio control is now at its margin.** It requires
production code to exceed 25% of `policy.rs`; after this round it is just above.
**The next round that documents `policy.rs` heavily will trip it**, and the
correct response is to budget prose (or move reasoning into the phase record),
never to lower the threshold. Recorded in both `19-SECURITY.md` and
`deferred-items.md` so it is met as a known threshold rather than rediscovered.

**`a_relocated_copy_of_the_stub_refuses_instead_of_acting` (`envelope_tracer`)
flaked once** on the first full-suite run with `Os { code: 26, kind:
ExecutableFileBusy, message: "Text file busy" }` at `tests/envelope_tracer.rs:185`.
Re-run three times in isolation: 6/6 green each time; green in the final gate.
A filesystem race on writing an executable stub, unrelated to this rule and not
fixed here. Worth noting beside the two documented `driver_reattach` flakes.

## What remains uncovered

1. **`T-19-86` — OPEN at `high`**, by explicit user scoping decision. A governed
   program's own operand reaching `classify_git`'s denylist default arm. Four
   rows still exit 0; control green and unmodified.
2. **`T-19-91` — OPEN at `high`.** Three arms: `reflog $S`, `reflog show $S`,
   `symbolic-ref $S` at exit 0 **with no second carrier**; plus the bare
   `git push $REF`, cwd-dependent and permitted in-namespace — a third arm, not
   "already fails closed". The second-carrier asymmetry (`push` has `pre-push`
   behind it; `reflog` and `symbolic-ref` have nothing) is unweakened.
3. **`T-19-96`** — a glob in a PUSH FLAG, one slot outside the decision region.
   Registered, pinned, not fixed.
4. **`T-19-74`** — core rows frozen at their measured verdicts.
5. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision.
6. **`T-19-17r`** — the bookkeeping gap stays **OUTSTANDING**. No `AR-19-13` row
   was added and the word "accepted" was not applied to it.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` are both sub-classes of it and both remain open at `high`.

## Self-Check: PASSED

* `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-19-SUMMARY.md` — FOUND
* `11745ca`, `b7f9684`, `7b0bcbe` — all FOUND in `git log`
* `git diff --stat 1d1229e..HEAD` touches exactly 4 files, all within
  `files_modified`; **neither `Cargo.toml` nor `Cargo.lock`** (`T-19-SC` holds)
* `19-SECURITY.md` and `deferred-items.md` diffs are **pure appends** (0 deleted
  lines), so no audit table, trail, risk log, sign-off or earlier appended
  record was touched
* Tree clean apart from the pre-existing untracked `.gsd/`
