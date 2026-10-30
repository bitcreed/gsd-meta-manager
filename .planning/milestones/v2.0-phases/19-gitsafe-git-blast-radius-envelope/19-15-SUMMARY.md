---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 15
subsystem: envelope
tags: [security, gap-closure, tokenizer, command-position, generative-testing]
status: complete
requires:
  - "19-14 (Rule A, the decision region; the two carry-forward RED rows and the 1527 total)"
  - "19-SECURITY.md audit 3 at `57a3cf3`, measured against the binary at `228e4bc`"
provides:
  - "`Token.word_splitting_flush` — set by `tokenize` for `( ) { }` only, on a word-in-progress condition"
  - "`policy::Segment` and `policy::split_segments_with_heads` — the per-segment head report, one scan"
  - "`policy::resolve_program_with_head` — a severed prefix is not a command position (Rule B)"
  - "the withdrawn textual formulation recorded in the code with both measurements that withdrew it"
  - "`SEVERED_PREFIXES` — a generative alphabet varying WHERE the split falls"
  - "brace-group and subshell `SHELL_LAYERS`, and the metacharacter floor over every alphabet"
affects:
  - "`/gsd-secure-phase 19` — still NOT cleared; T-19-86 and T-19-91 remain open at high"
tech-stack:
  added: []
  patterns:
    - "a decision region comes from the SAME scan the classifier runs, never a second scan"
    - "a rule about position reads no name, no substring and no length"
    - "a withdrawn formulation is recorded in the code with its measurements, not deleted"
    - "a control is committed RED before the fix, in its own commit"
key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - tests/envelope_expansion_slots.rs
    - tests/envelope_wrapper_class.rs
    - tests/envelope_command_position.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "Rule B is POSITIONAL; the substring/prefix/suffix/length formulation stays withdrawn on two measurements"
  - "The opener (`{`, `(`) is excluded, which is what keeps command substitutions classified"
  - "`split_segments` is DEFINED OVER `split_segments_with_heads` so there is one walk of the tokens"
  - "The false T-19-87 test is DELETED with a tombstone rather than re-worded"
  - "The bare-brace row is spelled WITHOUT its binding, because the bound spelling is already refused under `hook_bypass_blocked` and would be a control that could not fail on its class"
metrics:
  duration: "~1h"
  completed: 2026-08-29
actuals:
  tokens: 82000
  tasks: 2
  commits: 2
---

# Phase 19 Plan 15: Rule B — a severed prefix is not a command position Summary

**Closed the two rows 19-14 deliberately left RED, positionally: `tokenize` now records
that a `{ } ( )` boundary severed a word, and a segment whose immediately preceding
operator is a severing closer is refused when it resolves a governed program — with
`SEPARATORS` untouched and no character's meaning changed.**

## READ THIS FIRST — what is NOT closed

**`/gsd-secure-phase 19` is NOT cleared by this plan.**

1. **`T-19-86`** — a governed program's own operand naming a governed command
   (`git submodule foreach …`, `git rebase -x …`, `git bisect run …`,
   `git -c alias.p=…`). Untouched, unremediated, **OPEN at `high`** by explicit user
   scoping decision. All four rows still exit 0 and
   `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted` is
   unmodified and green.
2. **`T-19-91`** — a git classifier's own decision operand assembled by expansion, for
   every verb but `config`. **Registered OPEN at `high`** exactly as 19-14 wrote it,
   including the recorded fact that **`reflog` and `symbolic-ref` have NO `pre-push`
   and NO `pre-commit` second carrier** — git runs no hook for either. `git reflog $S`,
   `git reflog show $S` and `git symbolic-ref $S` are measured PERMITTED and pinned.
   `git push $REF` is refused only through the cwd-dependent no-refspec path
   (`T-19-80`), which is why the repository-free `git push origin $REF` is the pinned
   half.
3. **`T-19-74`** — narrowed at exactly one disclosed spelling by 19-14; the core is
   unmoved. `env $X push --force origin main` and
   `X=git; env $X push --force origin main` are still permitted.
4. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open, unaccepted, untouched.

**`T-19-87` and `T-19-89` ARE closed**, across 19-14 and 19-15 together — and never
write an unqualified "T-19-60 is closed": `T-19-86` and `T-19-91` are both sub-classes
of it and both remain open.

## The carry-forward — confirmed RED before anything was written

This was the plan's first action, before a line of test or production code. Verbatim,
against 19-14's post-state at `cae2793`:

```
running 2 tests
test the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red ... FAILED
test the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red ... FAILED

---- the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:232:5:
assertion `left == right` failed: `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin`
must be REFUSED. ... stdout:  stderr:
  left: 0
 right: 2

---- the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:232:5:
assertion `left == right` failed: `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin`
must be REFUSED. ... stdout:  stderr:
  left: 0
 right: 2

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 24 filtered out
```

Had either been green, Rule B would have been a rule nobody had shown was needed, and
the correct action was to report it as a finding rather than proceed. Both were red.

**The transition — the one number a run that did nothing cannot forge:**

| Row | before (`cae2793`) | after (`af72137`) |
|---|---|---|
| `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin` | exit 0 | **exit 2**, `envelope_assertion_failed` |
| `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin` | exit 0 | **exit 2**, `envelope_assertion_failed` |

Both test names are GREEN in the final run and neither was renamed, weakened or
`#[ignore]`d.

## The gate

**Command: `rtk proxy cargo test --no-fail-fast`.** A plain `cargo test` fail-fasts at
`tests/driver_reattach.rs` and `envelope_*` sorts after `driver_*`, so an unqualified
run never executes a single envelope suite and reports a baseline-identical count.
`rtk proxy` is required because the RTK hook strips the `warning:` and `test result:`
lines any count is read from (D-34).

| | passed | failed | ignored | **passed + failed** |
|---|---|---|---|---|
| 19-14's recorded post-state (`cae2793`) | 1525 | **2** (the two carry-forward rows) | 13 | **1527** |
| after this plan (`af72137`) | **1533** | **0** | 13 | **1533** |

`1533 > 1527`, strictly. Failure count **0**, which is inside the gate's "0 or 2". No
failure of any kind appeared, so the `driver_reattach` pair passed in this run (their
flake mode B, as in 19-14's own gate run). They were neither fixed nor touched.

### Per-binary counts (post-state)

| Binary | passed | failed |
|---|---|---|
| `--test envelope_expansion_slots` | **32** | 0 |
| `--test envelope_command_position` | **18** | 0 |
| `--test envelope_wrapper_class` | **19** | 0 |
| `--lib envelope::policy` | **61** | 0 |
| `--lib envelope` | 184 | 0 |

`envelope_expansion_slots` goes 24→32 (six new tests, two carry-forward rows now
green); `envelope_command_position` goes 19→18 (the one deletion);
`envelope_wrapper_class` goes 18→19 (the severed-prefix property).

`cargo clippy -- -D warnings` exits 0. `cargo clippy --all-targets` reports **4**
warnings — the same four 19-13 and 19-14 recorded, three in `src/browser.rs` and one in
`src/project_creator.rs`, none in any file this plan touches. The count did not grow.

## Commits, in order

| # | SHA | What |
|---|---|---|
| 1 | `898d364` | `test(19-15)`: RED — the severed-prefix class and the last two alphabet axes |
| 2 | `af72137` | `fix(19-15)`: Rule B — a severed prefix is not a command position |

The RED state is its own commit, made before any production line moved: commit 1 has
**no `src/` hunks at all**.

## Task 1 — verbatim RED output

`rtk proxy cargo test --test envelope_expansion_slots` against the unfixed tree:

```
test result: FAILED. 27 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out

failures:
    rule_bs_disclosed_cost_is_pinned_as_a_pair_with_both_halves_alone_permitted
    the_re_homed_t_19_87_rows_carry_the_reason_identifiers_the_corrected_bound_implies
    the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red
    the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red
    the_severed_prefix_class_is_refused_wherever_the_author_chooses_to_split_it
```

Three of the five messages, verbatim:

```
---- the_severed_prefix_class_is_refused_wherever_the_author_chooses_to_split_it stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:232:5:
assertion `left == right` failed: `env -u ${C} git fetch origin` must be REFUSED.
stdout:  stderr:
  left: 0
 right: 2

---- rule_bs_disclosed_cost_is_pinned_as_a_pair_with_both_halves_alone_permitted stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:232:5:
assertion `left == right` failed: `ROOT=$(git rev-parse --show-toplevel) git status`
must be REFUSED. stdout:  stderr:
  left: 0
 right: 2

---- the_re_homed_t_19_87_rows_carry_the_reason_identifiers_the_corrected_bound_implies stdout ----
thread '...' panicked at tests/envelope_expansion_slots.rs:232:5:
assertion `left == right` failed: `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin`
must be REFUSED. stdout:  stderr:
  left: 0
 right: 2
```

The 27 that PASSED against the unfixed tree include the grouping rows
(`{ git status; }`, `( git status )`, `( git push --force origin main )`), the
opener-exclusion permits, the `(git status)&&git fetch origin` row, and the
`hook_bypass_blocked` disclosure row — so the file could not have been red because the
harness cannot observe anything.

`rtk proxy cargo test --test envelope_wrapper_class` against the unfixed tree:

```
test result: FAILED. 18 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

failures:
    a_governed_program_behind_a_severed_prefix_is_refused_wherever_the_split_falls
```

```
---- a_governed_program_behind_a_severed_prefix_is_refused_wherever_the_split_falls stdout ----
thread '...' panicked at tests/envelope_wrapper_class.rs:2472:17:
assertion `left == right` failed:

A GOVERNED PROGRAM BEHIND A SEVERED PREFIX WAS PERMITTED.

  command : C=GIT_CONFIG env -u ${C}_COUNT "nice" -n 10 git fetch origin
  base    : git fetch origin
  recipe  : prefix="C=GIT_CONFIG " layer=None depth=1 entry="env -u ${C}_COUNT" at=0
  got     : exit 0 reason permit(the guard answered nothing)
  seed    : 0x1912c0de5eed0060
  left: 0
 right: 2
```

**The invariance property, the permitted corpus, the decoy property, the
expansion-wrapper property and the forge-slot property were all GREEN in that same run**
— which is what shows the two new `SHELL_LAYERS` entries preserve invariance rather than
being narrowed to agree with the code.

## The rows, before and after

Driven in-process through `hooks::guard_in`, **one fresh `TempDir` envelope root per
row**, envelope directory walked afterwards where a ledger claim is made.

### The severed-prefix class

| Command | before | after | reason identifier |
|---|---|---|---|
| `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `env -u ${C} git fetch origin` (no literal fragment) | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `C=GIT_CONFIG_COU; env -u ${C}NT git fetch origin` (two chars) | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `C=GIT_CONFIG_COUN; env -u ${C}T git fetch origin` (one char) | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `env -u $(printf %s%s GIT_CONFIG _COUNT) git fetch origin` | exit 0 | **exit 2** | `envelope_assertion_failed` |
| `K=GIT_SSH; env -u ${K}_COMMAND git push --force origin main` | exit 2 `force_push_blocked` | **exit 2** | `envelope_assertion_failed` |
| **`ROOT=$(git rev-parse --show-toplevel) git status`** — the COST | exit 0 | **exit 2** | `envelope_assertion_failed` |

### The rows that must NOT move, measured green before and after

```
exit=0  git status                                    exit=0  DIR=$(mktemp -d)
exit=0  { git status; }                               exit=0  RUN_ID=$(uuidgen)
exit=0  ( git status )                                exit=0  CONFIG=$(cat cfg)
exit=0  (git status)&&git fetch origin                exit=0  COMMAND=$(which git)
exit=0  echo $(git rev-parse HEAD)                    exit=0  ROOT=$(git rev-parse --show-toplevel)
exit=0  git log --format=%h $(git rev-parse HEAD)     exit=0  env $X push --force origin main
exit=0  cd "$HOME"                                    exit=0  X=git; env $X push --force origin main

exit=2  git push --force origin main            [force_push_blocked]
exit=2  { git push --force origin main; }       [force_push_blocked]
exit=2  ( git push --force origin main )        [force_push_blocked]
```

The grouping rows are the mechanical proof that **no character changed meaning**:
`{ git status; }` and `( git status )` reach `git status`'s verdict, and their force-push
spellings reach `git push --force`'s — under the base's own identifier, not Rule B's.

### One severed spelling EXCLUDED from Rule B's evidence, disclosed

`C=GIT_CONFIG_COUNT; env -u ${C} git fetch origin` was measured at **exit 2 under
`hook_bypass_blocked`** against `cae2793`, before Rule B: binding a WHOLE envelope key
name in the same command line is refused on its own account by `resolve_program` step 2b
(`T-19-81`), before resolution reaches the severed fragment at all.

**The plan spells the "no literal fragment" row with that binding attached.** Written
that way it would have been green before the fix and green after it — a control that
cannot fail on its own class, which is the exact defect four rounds of this phase have
been about. It is therefore pinned at its true identifier in a test of its own
(`the_severed_spelling_that_binds_a_whole_key_is_already_refused_for_another_reason`),
and Rule B's no-fragment row is the binding-free `env -u ${C} git fetch origin` — which
is the spelling the plan's own prohibition names, and which was measured exit 0 before
and exit 2 after. See Deviations.

## The withdrawn textual formulation, with both measurements

Recorded in `resolve_program_with_head`'s doc in `src/envelope/policy.rs`, in the header
of `tests/envelope_expansion_slots.rs` section 12, in the `SEVERED_PREFIXES` doc and in
the new `19-SECURITY.md` subsection — four places, because a later reader tempted by it
needs to meet the measurement wherever they arrive.

An earlier draft keyed Rule B on the literal fragment being a substring of an
`ENVELOPE_ENV_KEYS` entry. It was withdrawn on:

1. **Evadable — move the split point.** `C=GIT_CONFIG_COU; env -u ${C}NT …` leaves the
   two-character fragment `NT`; `${C}T` leaves one character; and the bare
   `env -u ${C} …` leaves none at all. Any minimum length is a floor an author ducks
   under by cutting one character further left. **All four spellings are measured
   refused by the positional rule** (table above).
2. **Unshippable — it refuses ordinary shell.** `ROOT`, `DIR`, `RUN`, `CONFIG` and
   `COMMAND` all sit inside envelope key names, `GSD_MM_RUN_ID` included — the key 19-14
   added. Each of `ROOT=$(git rev-parse --show-toplevel)`, `DIR=$(mktemp -d)`,
   `RUN_ID=$(uuidgen)`, `CONFIG=$(cat cfg)` and `COMMAND=$(which git)` would be refused
   on **every Bash tool call**. **All five are measured exit 0 after Rule B** and pinned.

The positional rule refuses the first set and permits the second, and it has nothing to
floor and nothing to duck under, because it decides about command position.

## The ONE pre-existing test deleted, named in advance

`the_brace_expansion_spelling_is_a_residual_this_plan_does_not_close`, in
`tests/envelope_command_position.rs`.

**Why.** It asserted, as the bound on `T-19-87`'s damage: *"the fragmentation does not
hide a refused git command, because the segment that carries the command still resolves
it."* `19-SECURITY.md`'s third audit measured that bound **FALSE one word to the right**
— the fragmentation hides `env -u GIT_SSH_COMMAND`, which removes `IdentitiesOnly=yes`,
`IdentityAgent=none` and `-F /dev/null` (D-16), and that harm lands whether or not the
git command behind it is refused. A bound stated on the git command was a bound about
the wrong half of the line.

**Deleted rather than re-worded**, with a tombstone in its place naming where its rows
went and why. A test asserting a wrong bound is worse than no test, and re-wording it
would have been a bound wider than the mechanism for the second time in the same file.

| Row | old verdict | new verdict | Justification |
|---|---|---|---|
| `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin` | **PERMITTED, exit 0** | **REFUSED, `envelope_assertion_failed`** | It is the carry-forward row itself. The bound that justified pinning it permitted was measured false; the line is a live layer-3 bypass. |
| `env -u ${K}_COMMAND git push --force origin main` | **REFUSED, `force_push_blocked`** | **REFUSED, `envelope_assertion_failed`** | The segment is now refused for an unresolvable command position **before** `classify_git` is reached, so `force_push_blocked` is no longer the reason it is refused. Asserting the old identifier would assert a mechanism that no longer runs for this line. |

Both rows now live in `tests/envelope_expansion_slots.rs::the_re_homed_t_19_87_rows_carry_the_reason_identifiers_the_corrected_bound_implies`.

**No other pre-existing test row was edited, and no other went red.**
`the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`,
`the_t_19_74_residual_is_permitted_and_the_cost_of_closing_it_is_measured_beside_it`,
`the_bounds_of_the_t_19_74_residual_are_refused_which_is_what_makes_it_narrow`,
`the_residual_begins_exactly_at_the_command_line_boundary`,
`resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`,
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`,
`every_command_behind_a_separator_is_its_own_segment`, the tokenizer's quoting table, the
invariance property, the permitted corpus, the decoy property and 19-14's forge-slot
property are all green and unmodified.

## What the fix actually is

Three edits and two registrations. **`SEPARATORS` is byte-for-byte unchanged**, verified
by diff, and no character's meaning changed.

1. **`Token.word_splitting_flush`** — set by `tokenize`'s separator arm from the state it
   already has, read BEFORE `flush!` clears `started`. Computed for `(`, `)`, `{` and `}`
   **only**; `false` for `;`, `|`, `&`, newline and every ordinary word. The doc records
   the discriminator and the cases it tells apart: `${X}push`, `$(true)push`, `${C}NT` and
   `${C}` have a word in progress at every boundary, `{ cmd; }` and `( cmd )` at neither
   — because bash requires whitespace after `{` and a `;` or newline before `}`. It also
   records why the condition is a word BEFORE the character (a word after it would mark
   `(cd /tmp && ls)` and refuse an ordinary subshell), and that a substitution inside
   double quotes is consumed by the quote loop and never reaches the arm at all.
2. **`policy::Segment` + `policy::split_segments_with_heads`** — segments plus, per
   segment, whether the operator IMMEDIATELY preceding it was a severing closer.
   **`split_segments` is DEFINED OVER it**, so its signature and behaviour are unchanged
   and there is **one walk of the tokens**. That is deliberate: a region derived from a
   second scan is the defect that has sat one slot over four times in this phase.
3. **`policy::resolve_program_with_head`** — refuses a `Governed`/`NestedPayload`
   resolution behind a head that is not a command position, under
   `EnvelopeAssertionFailed`, with a detail that names the shape and never quotes the
   command back (SAFE-04). `resolve_program` is that function with the head treated as
   real, so every existing caller and unit test is unchanged. Threaded in `hooks.rs` from
   `guard_in`'s split **and** from the `NestedPayload` arm's re-split — the path a
   `sh -c '…'` payload takes, whose braces are literal until the payload is re-split.
   `NoProgram` and `Ungoverned` pass through, and step 1's envelope-key refusal keeps its
   own more-specific `HookBypassBlocked` identifier.

**The opener is excluded and the code says why:** after `{` is the parameter expansion's
variable name, never a governed program; after `(` is the substitution's own contents,
which IS a command position and keeps being classified. "Immediately preceding" is
equally load-bearing — in `(git status)&&git fetch origin` the second segment's last
preceding operator is `&&`.

**No wrapper name and no wrapper flag was added anywhere.**
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` is green.

### `T-19-89` — the last two axes

* **`SHELL_LAYERS`** gains `BraceGroup` (`{ …; }`) and `Subshell` (`( … )`) — the two
  shapes audit 3 named absent and the two that exercise the splitter. Both respect
  `inner_quotes` so no illegal shell is emitted, and both preserve invariance, which the
  enumerated grouping rows assert directly.
* **`SEVERED_PREFIXES`** — a new alphabet with a refusal property of its own (a severed
  prefix is legitimately STRICTER than the unwrapped base, so invariance is the wrong
  comparison, exactly as the file already records for `GIT_CONFIG_COUNT=0` and
  `DECOY_OPERANDS`). Its eight entries vary **where the split falls**: trailing fragment,
  no fragment, one- and two-character tails, the substitution spelling, plus an entry
  carrying no envelope key name at all (`sudo -u $(id -un)`) that would be permitted if
  the rule were textual after all. Its doc states that varying the fragment's TEXT would
  be the withdrawn formulation wearing a property's clothes. Its bases are **innocuous
  permitted git and forge commands**, with a floor asserting each is permitted unwrapped
  — a corpus of `git push --force` bases would have been green for the base's reason and
  could not have failed on this class.
* **The per-alphabet metacharacter floor** now covers `SHELL_LAYERS` (through the
  spelling each layer produces) and `SEVERED_PREFIXES` as well as the four 19-14 added,
  completing the set audit 3's axis table names.

### Measured generated-case counts (post-fix)

```
severed-prefix corpus: 64 generated cases over 8 split points and 4 bases,
                       ≥20 distinct chains, 64/64 carrying a live expansion
                       seed: 0x1912c0de5eed0060
```

## Deviations from Plan

### 1. [Rule 1 — the plan's spelling would have been a vacuous control] The bare-brace row is written WITHOUT its binding

- **Found during:** Task 1, measuring every candidate row against `cae2793` before
  writing a single assertion.
- **Issue:** Task 1's action names the "no literal fragment" row as
  `C=GIT_CONFIG_COUNT; env -u ${C} git fetch origin` and requires it to assert
  `envelope_assertion_failed`. **Measured, that line is already exit 2 under
  `hook_bypass_blocked`** at `cae2793`: `resolve_program` step 2b refuses a leading
  assignment whose VALUE spells a whole envelope key, before resolution reaches the
  severed fragment. Written as the plan spells it, the row would have been refused
  before the fix and after it — a control that cannot fail on its own class, which is
  precisely the certification defect four rounds of this phase have been about, and it
  would also have asserted the wrong identifier.
- **Fix:** the Rule B row is the binding-free `env -u ${C} git fetch origin` — which is
  the spelling **the plan's own prohibition names** ("it cannot see the bare
  `env -u ${C} git fetch origin` at all, which has no literal fragment"), and whose
  binding-from-a-previous-command-line shape is exactly what `T-19-74` records as out of
  the guard's reach. Measured exit 0 before, exit 2 `envelope_assertion_failed` after.
  The plan's spelling is pinned separately at its true identifier in
  `the_severed_spelling_that_binds_a_whole_key_is_already_refused_for_another_reason`,
  so the interaction is disclosed rather than silently dropped.
- **Files modified:** `tests/envelope_expansion_slots.rs`
- **Commit:** `898d364`

### 2. [Rule 2 — completeness] Extra rows and one extra alphabet entry inside the plan's own class

Rows the plan's rule implies but does not enumerate, each measured first: the
grouping rows in their REFUSED spellings (`{ git push --force origin main; }` and
`( git push --force origin main )`, so "no character changed meaning" is pinned on both
sides of the verdict rather than only the permitted side), and the `SEVERED_PREFIXES`
entries `sudo -u $(id -un)`, `timeout ${T}s` and `made-up-wrapper-9000 --flag ${W}x` —
the first because a rule that refused the key-bearing spellings while permitting a
key-free one would be textual after all, the other two so the alphabet is not one
wrapper counted eight times.

### 3. [scope note] `resolve_program` wraps rather than is wrapped

The plan asks for `resolve_program(segment)` to be *defined as* the head-taking sibling
with the head treated as real. It is implemented the other way round —
`resolve_program_with_head` calls `resolve_program` and post-filters `Governed` /
`NestedPayload`. The two are behaviourally identical and there is still exactly one
implementation of the step machinery and one scan. The direction chosen keeps
`resolve_program`'s body and its `pub fn resolve_program(` anchor **byte-identical**,
which is what `resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`
reads; inverting it would have moved the disclosure doc off the anchor that control
depends on. Recorded rather than absorbed.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema change was
introduced; the diff narrows a decision surface and adds tests.

## Self-Check: PASSED

Verified against disk and git, not asserted:

- Files exist: `src/envelope/policy.rs`, `src/envelope/hooks.rs`,
  `tests/envelope_expansion_slots.rs`, `tests/envelope_wrapper_class.rs`,
  `tests/envelope_command_position.rs`, `19-SECURITY.md`, `deferred-items.md`, this
  SUMMARY.
- Commits exist and are in order: `898d364` (RED, **no `src/` hunks**), then `af72137`.
- `git diff cae2793..HEAD --stat` touches **only** files in the plan's `files_modified`;
  `Cargo.toml` and `Cargo.lock` are absent (`T-19-SC` holds phase-wide).
- The diff contains **0** hunks touching the `SEPARATORS` constant or the
  `pub fn split_segments(` signature line.
- `19-SECURITY.md`'s diff is **117 insertions, 0 deletions** — one appended subsection
  after 19-14's and nothing else. No audit table, trail, risk log, sign-off line or any
  part of 19-14's subsection was modified.
- No file was deleted by either commit.
- Final gate re-run after every edit: 1533 passed / 0 failed / 13 ignored,
  `full_suite_exit=0`.
