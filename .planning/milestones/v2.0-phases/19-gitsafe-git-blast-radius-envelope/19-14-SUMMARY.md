---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 14
subsystem: envelope
tags: [security, gap-closure, classifier, expansion, generative-testing]
status: complete
requires:
  - "19-13 (the command-position rule and `Token.expansion`, which Rule A reads)"
  - "19-SECURITY.md audit 3 at `57a3cf3`, measured against the binary at `228e4bc`"
provides:
  - "`policy::expansion_in_decision_region` — the decision-region rule as a pure function over `&[Token]`"
  - "three index primitives: `subcommand_word_indices`, `scan_config`/`config_key_operand_index`, `scan_gh_api`"
  - "`scan_leading`'s unreadable-config-key refusal"
  - "`GSD_MM_RUN_ID` in `ENVELOPE_ENV_KEYS`, and the drift pin re-sourced through `with_run_id`"
  - "`tests/envelope_expansion_slots.rs` — round-4 reproducers, the T-19-91 pins and the two carry-forward rows"
  - "the Rule A half of the generative alphabets plus a forge-slot property with a fresh root per case"
  - "a post-state suite total of 1527 (`passed + failed`) for 19-15 to gate against"
affects:
  - "19-15 (Rule B) — inherits two RED rows and the 1527 total"
tech-stack:
  added: []
  patterns:
    - "every index in a decision region is reported by the scan the classifier itself runs"
    - "a no-ledger-line claim is observed by WALKING a fresh envelope root, never derived from one expected path"
    - "a control is committed RED before the fix, in its own commit"
key-files:
  created:
    - tests/envelope_expansion_slots.rs
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "Design option (a) — a decision REGION — over option (b), refusing any expansion in a governed segment, on the measured cost that (b) refuses ordinary commit messages and PR titles"
  - "The `gh api` endpoint index comes from that arm's OWN scan, not from `subcommand_words`, because the two scans disagree about which words are flags"
  - "T-19-87's two severed-prefix rows are left RED, as the mechanism proving Rule A and Rule B are separately load-bearing"
  - "T-19-91 is measured, pinned and registered OPEN rather than closed — round discipline, explicitly NOT a second-carrier argument"
  - "The two carry-forward rows are ONE TEST EACH, because the plan's own gate arithmetic requires two failing names"
metrics:
  duration: "~1h"
  completed: 2026-08-29
actuals:
  tokens: 96000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 14: Rule A — the decision region Summary

**Restored `Token.expansion` at the classifier decision boundary: T-19-88 and T-19-90
closed, six of T-19-87's eight rows closed with `SEPARATORS` untouched, and two rows
deliberately left RED for 19-15.**

## READ THIS FIRST — what is NOT closed

**`/gsd-secure-phase 19` is NOT cleared by this plan, and will not be cleared by 19-15
either.**

1. **`T-19-86`** — a governed program's own operand naming a governed command. Untouched,
   unremediated, **OPEN at `high`** by explicit user scoping decision. Its four rows
   (`git submodule foreach …`, `git rebase -x …`, `git bisect run …`, `git -c alias.p=…`)
   were green before Rule A and are green after it, all four still at exit 0, and
   `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted` is unmodified.
2. **`T-19-87` — NOT closed. Six of eight measured rows.** The two severed-prefix rows are
   written, observed RED, and left RED for 19-15.
3. **`T-19-89` — NOT closed. The Rule A half only.** `SHELL_LAYERS` still cannot draw
   `{ …; }` or `( … )`, and there is no severed-prefix alphabet. 19-15 completes it.
4. **`T-19-91` — registered OPEN at `high`.** `config`'s key operand is closed; `git reflog
   $S`, `git reflog show $S` and `git symbolic-ref $S` are measured PERMITTED and pinned.
5. **`T-19-74`** — narrowed at exactly one disclosed spelling; the core is unmoved.
6. **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open, unaccepted, untouched.

## The gate

**Command: `rtk proxy cargo test --no-fail-fast`.** A plain `cargo test` fail-fasts at
`tests/driver_reattach.rs`, and `envelope_*` sorts after `driver_*`, so an unqualified run
never executes a single envelope suite. `rtk proxy` is required because the RTK hook strips
the `warning:` and `test result:` lines any count is read from (D-34).

| | passed | failed | ignored | **passed + failed** |
|---|---|---|---|---|
| baseline (`c595141`) | 1495 or **1497** | 2 or **0** | 13 | **1497** |
| after this plan | **1525** | **2** | 13 | **1527** |

The baseline is bimodal (the flaky `driver_reattach` pair), which is why the constant is
`passed + failed`. The baseline run taken at `c595141` for this plan reported the
`1497 / 0 / 13` mode.

### THE HANDOFF NUMBER FOR 19-15

> **The exact post-state `passed + failed` total is `1527`.**
> 19-15's gate is `passed + failed` **strictly greater than 1527**, with 0 or 2 failures, all
> `driver_reattach` names.

### Failure names, classified

| Test | Binary | Classification |
|---|---|---|
| `the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red` | `envelope_expansion_slots` | **carry-forward — expected RED** |
| `the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red` | `envelope_expansion_slots` | **carry-forward — expected RED** |

The `driver_reattach` pair passed in this run (flake mode B). Failure count 2 is inside the
gate's `2 or 4`. No third kind of name appeared.

### Per-binary counts (post-state)

| Binary | passed | failed |
|---|---|---|
| `--test envelope_expansion_slots` | 24 | **2 (the two carry-forward rows only; every other row green)** |
| `--test envelope_command_position` | 19 | 0 |
| `--test envelope_wrapper_class` | 18 | 0 |
| `--lib envelope::policy` | 61 | 0 |
| `--lib envelope` | 184 | 0 |
| `envelope_wrapper_bypass` / `envelope_pr_cap` / `envelope_hook_refusals` / `envelope_wiring` / `envelope_credential` / `envelope_advisory` / `envelope_tracer` | 13 / 11 / 7 / 14 / 6 / 10 / 6 | 0 |

`cargo clippy -- -D warnings` exits 0. `cargo clippy --all-targets` reports **4** warnings —
the same four 19-13 recorded, all in `src/project_creator.rs` and `src/**` test modules this
plan does not touch. The count did not grow.

## Commits, in order

| # | SHA | What |
|---|---|---|
| 1 | `2d5514a` | `test(19-14)`: RED — the decision-region reproducers, the carry-forward and T-19-91 |
| 2 | `f279a62` | `test(19-14)`: RED — the Rule A alphabets and the forge-slot property |
| 3 | `8f4cbe3` | `fix(19-14)`: Rule A — restore the expansion bit at the decision boundary |

Each RED state is its own commit, made before any production line moved. Commit 1's only
`src/` hunks are inside `#[cfg(test)] mod tests` or are doc comments.

## Task 1 — verbatim RED output

`rtk proxy cargo test --test envelope_expansion_slots` against the unfixed tree:

```
test result: FAILED. 9 passed; 16 failed; 0 ignored; 0 measured; 0 filtered out

failures:
    a_config_key_operand_assembled_by_expansion_is_refused
    a_forge_first_subcommand_word_assembled_by_expansion_is_refused_and_writes_no_ledger_line
    a_forge_second_subcommand_word_assembled_by_expansion_is_refused_and_writes_no_ledger_line
    a_git_verb_assembled_by_expansion_is_refused_in_every_measured_spelling
    a_leading_config_option_whose_key_cannot_be_read_is_refused
    an_api_endpoint_assembled_by_expansion_is_refused_and_writes_no_ledger_line
    an_api_endpoint_displaced_out_of_the_first_two_subcommand_words_is_still_refused
    an_api_flag_that_is_dash_initial_with_an_unreadable_key_half_is_a_separate_row_on_purpose
    an_api_flag_whose_flag_ness_cannot_be_read_is_refused_and_writes_no_ledger_line
    an_api_method_assembled_by_expansion_is_refused_and_writes_no_ledger_line
    removing_the_run_id_key_is_refused
    the_brace_and_paren_spellings_that_land_in_a_verb_slot_are_refused
    the_brace_spelling_on_the_forge_path_is_refused_and_writes_no_ledger_line
    the_composed_three_layer_line_is_refused_through_its_verb_slot_half
    the_two_severed_prefix_rows_are_carried_forward_to_19_15_and_are_expected_red
    the_verbs_with_no_second_carrier_are_the_reason_this_class_is_rated_high
```

(The last name was later split into one test per row; see "Deviations".) Three representative
messages, verbatim:

```
---- a_git_verb_assembled_by_expansion_is_refused_in_every_measured_spelling stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:230:5:
assertion `left == right` failed: `V=push; git $V --force origin main` must be REFUSED.
`19-SECURITY.md`'s third audit measured this line at exit 0 against the built binary:
`classify_segments` collapses each `Token` to its `text` before either classifier runs,
so `Token.expansion` is structurally unavailable to the words the decision turns on.
stdout:  stderr:
  left: 0
 right: 2

---- a_forge_second_subcommand_word_assembled_by_expansion_is_refused_and_writes_no_ledger_line stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:259:5:
assertion `left == right` failed: `P=create; gh pr $P --title x` must be REFUSED.
stdout:  stderr:
  left: 0
 right: 2

---- an_api_endpoint_displaced_out_of_the_first_two_subcommand_words_is_still_refused stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:259:5:
assertion `left == right` failed: `E=pulls; gh api -f title=x repos/o/r/$E` must be REFUSED.
stdout:  stderr:
  left: 0
 right: 2
```

The nine that PASSED against the unfixed tree are the anti-vacuity controls, the walk's
positive control, the `T-19-91` pins and the six paired-allow tests — so the file could not
have been red because the harness cannot observe anything.

The drift pin, `rtk proxy cargo test --lib envelope::policy`:

```
---- envelope::policy::tests::every_envelope_key_the_child_environment_actually_carries_is_covered_by_the_constant stdout ----
thread '...' panicked at src/envelope/policy.rs:3253:13:
`GSD_MM_RUN_ID` is SET in the driven child's environment by `cred::build_env_in` but is not
covered by `ENVELOPE_ENV_KEYS`, so a command that reassigns or removes it is not refused.
**Add the key to the constant.** ... Covered set: ["GIT_CONFIG_COUNT", ..., "GSD_MM_ENVELOPE_PROJECT_ROOT"]

test result: FAILED. 60 passed; 1 failed; 0 ignored; 0 measured; 1071 filtered out
```

**Floor 5 passed while the coverage loop failed**, which is the whole point: the re-sourced
pin can now SEE the entry, so it fails on coverage rather than being blind to it. Re-sourcing
back to a bare `build_env_in(...)` turns floor 5 red.

### The walked envelope listing (measured, not described)

Printed by the positive control after a PERMITTED `gh pr create --title x`:

```
positive control — walked envelope root after a PERMITTED `gh pr create`:
  alpha/pr-ledger.ndjson (103 bytes)
  ledger lines found: 1
```

Every refused forge row asserts this walk returns nothing. Without the listing above those
assertions could all be passing because the walk is blind.

### Measured `T-19-91` verdicts (unfixed tree, `c595141`)

```
exit=0  git reflog $S                                             <- PERMITTED, registered OPEN
exit=0  git reflog show $S                                        <- PERMITTED, registered OPEN
exit=0  git symbolic-ref $S                                       <- PERMITTED, registered OPEN
exit=2  git symbolic-ref HEAD $R  [force_push_blocked]            <- already fails closed
exit=2  git push origin $REF      [push_outside_namespace]        <- already fails closed
exit=2  git push $REF             [push_outside_namespace]        <- already fails closed, repo-dependent
```

**`reflog` and `symbolic-ref` have NO `pre-push` and NO `pre-commit` second carrier.** Git
runs no hook for either, and `classify_reflog`'s own refusal text records that the reflog is
the recovery path for every other destructive git operation. Only `git push` has a hook
behind it, and its refspec operand already fails closed. The reason `config` is closed and
these are not is **round discipline plus provenance** — `git ${X}config core.hooksPath
/tmp/x` is a row in audit 3's own measured list, while these were found while checking this
plan — and **not** a second-carrier argument.

`git push $REF` is recorded rather than pinned as a live row: it is the one shape
`push_needs_resolved_dests` answers `true` for, which makes the guard shell out to `git` in
the caller's working directory, so its verdict depends on the repository the test runs in
(`T-19-80`). The repository-free half is the pinned `git push origin $REF`.

## Task 2 — verbatim RED output

`rtk proxy cargo test --test envelope_wrapper_class` against the unfixed tree:

```
test result: FAILED. 13 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out

failures:
    a_forge_subcommand_slot_carrying_an_expansion_is_refused_in_every_generated_slot
    a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position
    a_wrapper_prefix_carrying_an_expansion_is_refused_in_every_generated_position
    the_decoy_rule_discriminates_rather_than_blanket_denying_the_shape
    the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command
```

```
---- the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command stdout ----
thread '...' panicked at tests/envelope_wrapper_class.rs:454:9:
assertion `left == right` failed: the UNWRAPPED base `git $V --force origin main` must
itself be refused. If this row is red, every invariance assertion below compares a permit
against a permit and proves nothing — the harness cannot observe a denial at all.
Got reason id: permit(the guard answered nothing)
  left: 0
 right: 2

---- a_forge_subcommand_slot_carrying_an_expansion_is_refused_in_every_generated_slot stdout ----
thread '...' panicked at tests/envelope_wrapper_class.rs:2171:21:
assertion `left == right` failed:

A FORGE DECISION WORD CARRYING AN EXPANSION WAS PERMITTED.

  command : gh $P create --title x
  slot    : First
  spelling: 0 (0=$W 1=${W} 2=`true`W)
  depth   : 0
  got     : exit 0 reason permit(the guard answered nothing)
  seed    : 0x1912c0de5eed0060
...
  left: 0
 right: 2

---- the_decoy_rule_discriminates_rather_than_blanket_denying_the_shape stdout ----
thread '...' panicked at tests/envelope_wrapper_class.rs:934:5:
assertion `left == right` failed: `env -u git $X push --force origin main` must be REFUSED.
Got reason id: permit(the guard answered nothing)
  left: 0
 right: 2
```

Three of the five reds are the `floor 2` of an existing property — the floor that measures
every base refused UNWRAPPED. That is exactly the non-vacuity the widening is about: the
alphabets could not previously draw a `$`, so 1680 generated cases certified a fix that the
next audit walked around with two characters.

### Measured generated-case counts (post-fix, `--nocapture`)

```
refused corpus:            2100 generated cases,  937 distinct wrapper chains, 4 distinct D-24 reasons
permitted corpus:          1080 generated cases,  533 distinct wrapper chains
decoy corpus:              1170 generated cases,  830 distinct decoy chains, 13 decoy operands,
                                                  247 cases wrapped on both sides
expansion-wrapper corpus:   480 generated cases,  480 CARRYING A LIVE EXPANSION,
                                                  285 distinct chains, 8 wrapper entries
forge-slot corpus:           60 generated cases over 7 slots and 4 displacing options,
                                                  60 distinct commands
seed: 0x1912c0de5eed0060
```

**The counted expansion floor is 480 of 480**, over the 300 the plan sets, counted while
generating rather than inferred from alphabet sizes. The forge-slot property covers all
seven slots and all four displacing options, enforced by a slot-coverage floor.

## The audit-3 rows this plan closes

Driven in-process through `hooks::guard_in`, one fresh `TempDir` per row, envelope directory
walked afterwards.

| Command | before | after | reason identifier |
|---|---|---|---|
| `V=push; git $V --force origin main` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `V=stash; git $V` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `V=update-ref; git $V -d refs/heads/main` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `P=pr; gh $P create --title x` | exit 0, no ledger line | **exit 2**, walk empty | `envelope_assertion_failed` |
| `` gh `true`pr create --title x `` | exit 0, no ledger line | **exit 2**, walk empty | `envelope_assertion_failed` |
| `git ${X}push --force origin main` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `git $(true)push --force origin main` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `git ${X}stash` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `git ${X}update-ref -d refs/heads/main` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `git ${X}config core.hooksPath /tmp/x` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `gh ${X}pr create --title x` | exit 0, no ledger line | **exit 2**, walk empty | `envelope_assertion_failed` |
| `C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `env -u GSD_MM_RUN_ID git fetch origin` | exit 0 | **exit 2** | `hook_bypass_blocked` |

### The three region cells found while CHECKING this plan

Not measured by any audit; same method, recorded at that provenance. Each was exit 0 before
and is `exit 2 [envelope_assertion_failed]` after.

| Cell | Command | Why the wrong scan missed it |
|---|---|---|
| the forge's SECOND subcommand word | `P=create; gh pr $P --title x` | `pr_command_label` matches on TWO words; a one-word region leaves this unmatched, uncounted, unparked — SAFE-06 bypassed rather than exceeded |
| the `-`-initial `api` flag | `F=f; gh api repos/o/r/pulls -$F title=x` and `-${F}` | begins with `-`, so neither one of the first two subcommand words, nor the method value, nor marker-initial |
| the DISPLACED `api` endpoint | `E=pulls; gh api -f title=x repos/o/r/$E` and the `-H accept:x` spelling | the two forge scans disagree about which words are flags; an option value only the `api` scan skips pushes the endpoint to the THIRD word and out of the region entirely |

Plus `classify_config`'s key operand (`git config $K /tmp/x`, `${K}`, `set $K`) and the
`git -c` key half (`git -c $K commit -m x`, `git -c ${K}=/tmp/x commit -m x`).

**The defect is one shape recurring**: a decision region computed by a SECOND scan rather
than by the scan the classifier itself runs. It has now sat one slot over four times — the
wrapper operand, the governed program's own operand, the verb slot, and the `api` endpoint.
The fix answers it structurally: three extracted index primitives, and every index in the
region comes from the scan whose answer it guards.

## The cost, pinned from both sides

Design option (b) — refusing any expansion-carrying token anywhere in a governed segment — is
**rejected on a measured cost**. Every row below exits 0 after the fix and is pinned:

```
exit=0  git commit -m "use ${HOME} here"
exit=0  git commit -m "$MSG"
exit=0  git commit -m "fix: stop git push --force bypassing the guard"
exit=0  gh pr create --title 'fix $PATH handling' --body x
exit=0  gh pr create --title "$MSG" --body x
exit=0  gh api repos/o/r/pulls -f title="$T"        (and it is COUNTED — one ledger line)
exit=0  git -c user.name="$NAME" commit -m x
exit=0  git config user.email "$EMAIL"
exit=0  echo $(git rev-parse HEAD)
exit=0  git log --format=%h $(git rev-parse HEAD)
exit=0  cd "$HOME"
exit=0  env $X push --force origin main             (AR-19-10 core, unmoved)
exit=0  X=git; env $X push --force origin main      (AR-19-10 core, unmoved)
```

## The ONE pre-existing row this plan converts

| Row | File | Old verdict | New verdict |
|---|---|---|---|
| `env -u git $X push --force origin main` | `tests/envelope_wrapper_class.rs::the_decoy_rule_discriminates_rather_than_blanket_denying_the_shape` | **PERMITTED (exit 0)** | **REFUSED (exit 2), `envelope_assertion_failed`** |

**Justification.** The segment resolves `Governed { index: 2 }` on the decoy — ONE candidate,
so the ambiguity rule does not fire — and the verb `classify_git` would then judge is `$X`,
which is the decision word Rule A refuses. **This is a side effect of closing `T-19-88`, not
a decision to narrow AR-19-10.** The core of the accepted residual is unmoved and still
green, unmodified: `env $X push --force origin main`, `X=git; env $X push --force origin
main`, `echo $(git rev-parse HEAD)`, `cd "$HOME"`, and
`the_residual_begins_exactly_at_the_command_line_boundary`.

No other pre-existing row went red. `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`,
`the_t_19_74_residual_is_permitted_and_the_cost_of_closing_it_is_measured_beside_it`,
`the_bounds_of_the_t_19_74_residual_are_refused_which_is_what_makes_it_narrow`,
`resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`,
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`, the invariance
property, the permitted corpus and every test in `tests/envelope_command_position.rs` are
green and unmodified.

## Deviations from Plan

### 1. [Rule 3 — blocking] The carry-forward is TWO tests, not one

- **Found during:** Task 3, running the gate.
- **Issue:** Task 1's action says the two carry-forward rows go "in their own test". Written
  as one test, the binary reported `24 passed; 1 failed` — but the plan's own gate is read as
  "the failure count is **2 or 4**, and every failure name is a `driver_reattach` name or
  **one of the two carry-forward key-assembly test names**". That arithmetic only holds if
  each carried row is its own test name; with both in one test the full-suite count is 1 or 3
  and the gate cannot be satisfied at all. 19-15's mirror gate ("0 or 2 failures") has the
  same requirement.
- **Fix:** split into
  `the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red` and
  `the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red`, one row
  each, with the shared explanation as a section comment above both and a paragraph recording
  why the split is deliberate.
- **Files modified:** `tests/envelope_expansion_slots.rs`
- **Commit:** `8f4cbe3`

### 2. [Rule 2 — completeness] Extra rows added inside the plan's own region

Rows the plan's region definition implies but does not enumerate, each measured RED first and
each inside the stated region — no widening: `git -c ${K}=/tmp/x commit -m x` (the brace
spelling of the `-c` key), `git config set $K /tmp/x` (the subcommand form of the config key
operand), `gh api repos/o/r/pulls -X=$M -f title=x` (the `=`-attached method spelling, which
the plan names but did not list as a row), `GSD_MM_RUN_ID=other git fetch origin` (the
assignment form beside the `env -u` removal), `git reflog show $S` and
`git symbolic-ref HEAD $R` and `git push origin $REF` (extra `T-19-91` measurements, so the
residual's width is honest rather than assumed).

### 3. [scope note] `git push $REF` recorded rather than pinned

The plan asks for `git push $REF` to be measured and pinned. It was measured (exit 2,
`push_outside_namespace`) but is **not** a live pinned row, because it is the one shape
`push_needs_resolved_dests` answers `true` for and its verdict therefore depends on the
repository the test runs in (`T-19-80`, which this corpus explicitly avoids). Its measured
verdict is recorded here, in `19-SECURITY.md` and in `deferred-items.md`, and the
repository-free half of the same question — `git push origin $REF` — is pinned instead. This
is disclosure, not omission.

## What the fix actually is

Five edits, none of which names a wrapper, a wrapper flag or a git verb.
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` is green.

1. **Three index primitives**, so no region is computed by a second scan:
   `subcommand_word_indices` (with `subcommand_words` defined over it),
   `scan_config` + `config_key_operand_index` (with `classify_config` defined over it), and
   `scan_gh_api` (with `gh_api_posts_a_pull_request` defined over it). The third is a
   measured hole rather than symmetry, and its doc says so.
2. **`policy::expansion_in_decision_region`** — a pure function over `&[Token]` returning the
   first decision word that carries an expansion. Region: the git verb at the index
   `classify_git` slices to, plus `config`'s key operand; a forge's first two subcommand
   words; and for `gh api` the endpoint at the index its own scan reports, the method in both
   spellings, and any token that begins with an expansion marker **or** begins with `-` and
   carries an expansion before its first `=`.
3. **`scan_leading` refuses an unreadable config key half** — `$` or backtick before the
   first `=`. Textual, because that function is a pure argv function with two callers.
4. **One call site**, in `classify_segments`'s `Governed` arm, before both classifiers and
   before the ledger write.
5. **`GSD_MM_RUN_ID` in `ENVELOPE_ENV_KEYS`**, and the drift pin re-sourced through
   `with_run_id` with a floor that turns red if the source is narrowed back.

Plus: `a_verb_assembled_by_expansion_and_an_eval_are_both_denied` **renamed** to
`a_program_assembled_by_expansion_and_an_eval_are_both_denied` — it exercises `$TOOL push
--force`, which is step 3, the head. Its assertions are the only pin on that path, so it is
renamed rather than rewritten or deleted; a new test,
`a_decision_word_assembled_by_expansion_is_denied_on_the_git_verb_and_both_forge_slots`,
covers the class the old name claimed.

`SEPARATORS`, `split_segments`, the tokenizer's separator arm, `tests/envelope_command_position.rs`,
`Cargo.toml` and `Cargo.lock` are all untouched — verified by diff.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema change was
introduced; the diff narrows a decision surface and adds tests.

## Self-Check: PASSED

Verified against disk and git, not asserted:

- Files exist: `tests/envelope_expansion_slots.rs`, `src/envelope/policy.rs`,
  `src/envelope/hooks.rs`, `tests/envelope_wrapper_class.rs`, `19-SECURITY.md`,
  `deferred-items.md`, this SUMMARY.
- Commits exist and are in order: `2d5514a`, `f279a62`, `8f4cbe3`.
- `git diff c595141..HEAD` touches **no** part of `tests/envelope_command_position.rs`,
  `Cargo.toml` or `Cargo.lock`, and contains **0** hunks touching `SEPARATORS`,
  `fn tokenize` or `fn split_segments`.
- No file was deleted by any of the three commits.
- `19-SECURITY.md`'s diff is **144 insertions, 0 deletions** — one appended subsection and
  nothing else. No audit table, trail, risk log or sign-off line was modified.
