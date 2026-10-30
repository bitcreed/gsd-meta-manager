---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 13
subsystem: envelope
tags: [security, gap-closure, T-19-60, T-19-81, T-19-82, T-19-83, T-19-86, T-19-87]
status: complete
requires:
  - "19-11 — `resolve_program` and the structural wrapper resolution it replaced `words[0]` with"
  - "19-12 — the generative class-level corpus in `tests/envelope_wrapper_class.rs`"
provides:
  - "`resolve_program`'s command-position rule: head shortcut, candidate collection, refusal on two or more candidates behind a wrapper prefix"
  - "the unresolvable-expansion-prefix refusal (`T-19-81` class-level half)"
  - "the leading-assignment VALUE check against `envelope_env_key`, placed before the `NoProgram` return"
  - "`policy::forge_subcommand_names_a_governed_program` — the forge-side second layer, called before the ledger write"
  - "`classify_git`'s governed-verb refusal — the git-side second layer"
  - "`SSH_AUTH_SOCK`, `SSH_AGENT_PID`, `GSD_MM_ENVELOPE_PROJECT_ROOT` in `ENVELOPE_ENV_KEYS`, with an unfiltered drift pin"
  - "`tests/envelope_command_position.rs` — the audit-2 reproducer corpus"
  - "`DECOY_OPERANDS` and its refusal property in `tests/envelope_wrapper_class.rs`"
  - "`T-19-86` and `T-19-87` registered, pinned and disclosed"
affects:
  - "src/envelope/policy.rs"
  - "src/envelope/hooks.rs"
tech-stack:
  added: []
  patterns:
    - "structural command-position resolution — never a wrapper-name or wrapper-flag list (D-08)"
    - "residuals bounded on both sides: a disclosed limitation is pinned at its current verdict so closing it is a deliberate deletion"
    - "RED-before-fix, one commit per gate"
key-files:
  created:
    - tests/envelope_command_position.rs
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "`T-19-60` is closed for the WRAPPER-OPERAND sub-class only; `T-19-86` is registered, pinned and deferred"
  - "`/gsd-secure-phase 19` is NOT cleared by this plan alone"
  - "`T-19-87` (brace expansion fragments a command past the splitter) found in execution, pinned and deferred rather than fixed"
metrics:
  duration: "~50 min"
  completed: 2026-08-29
actuals:
  tokens: 7900
  tasks: 3
  commits: 3
---

# Phase 19 Plan 13: Command Position, the Envelope-Key Symmetry, and the Residual It Registers Summary

`resolve_program` now establishes **command position** — a governed candidate at the
head answers immediately, two or more behind a wrapper prefix are refused rather than
mis-indexed — closing `T-19-60`'s **wrapper-operand sub-class**, `T-19-81` on both
halves, `T-19-82`'s key-set gap, and `T-19-83`'s corpus vacuity; `T-19-86` and a newly
found `T-19-87` are registered, pinned at their current permitted verdicts, and left
open.

## Commits, in order

| # | SHA | What |
|---|---|---|
| 1 | `b5a211b` | **RED** — `tests/envelope_command_position.rs` + the widened drift pin |
| 2 | `2e1b799` | **RED** — the `DECOY_OPERANDS` alphabet and its property |
| 3 | `7ef7ba2` | **FIX** — the five code edits, the `T-19-86` disclosure and its registrations |

No commit creates a control and its fix together. Base: `a41e431`.

## The gate

**Baseline, measured at `a41e431` with `rtk proxy cargo test --no-fail-fast`:**

```
passed=1470 failed=2 ignored=13 passed_plus_failed=1472
```

**After, same command:**

```
passed=1495 failed=2 ignored=13 passed_plus_failed=1497
```

`passed + failed` 1472 → **1497**, strictly greater as the gate requires. Failures
number 2 and both names are the documented flaky pair:

```
a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step
a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired
```

Neither was touched (`deferred-items.md`, out of scope). The baseline is **bimodal** —
`1470/2/13` and `1472/0/13` are both documented on this host — so the constant
`passed + failed` is what the gate is read on, not the passed count.

**Per-binary counts, which a flake in another binary cannot forge:**

| Binary | Result |
|---|---|
| `--test envelope_command_position` | `ok. 19 passed; 0 failed; 0 ignored` |
| `--test envelope_wrapper_class` | `ok. 15 passed; 0 failed; 0 ignored` |
| `--lib envelope::policy` | `ok. 61 passed; 0 failed; 0 ignored` |

`cargo clippy -- -D warnings` exits 0. `cargo clippy --all-targets` reports 4 warnings,
all pre-existing and in `src/browser.rs` and `src/project_creator.rs` — unchanged count,
none in files this plan touched. `Cargo.toml` and `Cargo.lock` are untouched (`T-19-SC`).

## Task 1 — the RED output, verbatim

`rtk proxy cargo test --test envelope_command_position`, against the unfixed tree:

```
failures:
    a_force_push_behind_a_governed_decoy_operand_is_refused
    a_leading_assignment_naming_an_envelope_key_is_refused_and_the_boundary_is_pinned_beside_it
    a_second_wrapper_between_the_decoy_and_the_program_is_refused
    the_decoy_operand_spelled_as_an_absolute_path_is_refused_too
    the_decoy_shape_is_refused_even_where_no_second_carrier_exists
    the_decoy_that_clears_all_three_layers_with_the_credential_intact_is_refused
    the_forge_decoy_is_refused_before_any_ledger_line_is_written
    the_indirect_spelling_no_literal_match_can_find_is_refused_too
    the_measured_ssh_agent_restoration_line_is_refused
    the_sudo_spelling_of_the_decoy_is_refused

test result: FAILED. 8 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test envelope_command_position`
```

Two of the ten, in full, because they are the rows the whole round is about:

```
---- a_force_push_behind_a_governed_decoy_operand_is_refused stdout ----

thread 'a_force_push_behind_a_governed_decoy_operand_is_refused' panicked at
tests/envelope_command_position.rs:135:5:
assertion `left == right` failed: `env -u git git push --force origin main` must be
REFUSED. `19-SECURITY.md`'s second audit measured this line at exit 0 against the built
binary: a token that is not the effective program captured `resolve_program`'s index
because it is spelled as a governed program in a wrapper's operand slot, and the real
command became `argv[0]` of the classified argv. stdout:  stderr:
  left: 0
 right: 2

---- the_forge_decoy_is_refused_before_any_ledger_line_is_written stdout ----

thread 'the_forge_decoy_is_refused_before_any_ledger_line_is_written' panicked at
tests/envelope_command_position.rs:312:5:
assertion `left == right` failed: `env -u gh gh pr create --title x` must be REFUSED.
Measured at exit 0: two `gh` candidates, the first captured the index, and
`pr_command_label` then read a subcommand chain beginning with `gh` and recognised
nothing. stdout:  stderr:
  left: 0
 right: 2
```

The **8 passing** rows are the anti-vacuity controls (the unwrapped force push refuses;
the audit's own discriminator `env -u SOME_VAR git push --force origin main` refuses),
the nine allow rows including the two head-shortcut commit messages and the two PR
titles, `env -u git ls`, the `T-19-82` enabling half asserted PERMITTED, and the four
`T-19-86` residual rows. A red file that could not observe a denial at all would have
had these red too.

### The widened drift pin, RED

`rtk proxy cargo test --lib envelope::policy`:

```
thread 'envelope::policy::tests::every_envelope_key_the_child_environment_actually_carries_is_covered_by_the_constant'
panicked at src/envelope/policy.rs:2859:9:
at least one COVERED key must begin with neither `GIT_` nor `GH_`. This floor exists
because the deleted name filter would silently exclude a future `GITHUB_TOKEN`,
`GLAB_CONFIG_DIR` or `SSH_*` — and already excluded `GSD_MM_ENVELOPE_PROJECT_ROOT`.
Re-introducing that filter turns this red. Carried: [("SSH_AUTH_SOCK", true),
("SSH_AGENT_PID", true), ("GIT_SSH_COMMAND", false), ("GIT_CONFIG_GLOBAL", false),
("GIT_CONFIG_SYSTEM", false), ("GIT_ASKPASS", false), ("GIT_TERMINAL_PROMPT", false),
("GH_CONFIG_DIR", false), ("GSD_MM_ENVELOPE_PROJECT_ROOT", false),
("GIT_CONFIG_COUNT", false), ("GIT_CONFIG_KEY_0", false), ("GIT_CONFIG_VALUE_0", false)],
covered set: ["GIT_CONFIG_COUNT", "GIT_CONFIG_KEY_", "GIT_CONFIG_VALUE_",
"GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM", "GIT_ASKPASS", "GIT_TERMINAL_PROMPT",
"GIT_SSH_COMMAND", "GH_CONFIG_DIR"]

test result: FAILED. 58 passed; 1 failed; 0 ignored; 0 measured; 1071 filtered out
```

**This is the pin's fail-first evidence and it is worth reading closely.** The failure
message prints the entire carried set, so the RED state itself records that
`cred::build_env_in` carries **12** entries, that exactly **2** of them are removals
(`true` = `value.is_none()`), and that three — `SSH_AUTH_SOCK`, `SSH_AGENT_PID`,
`GSD_MM_ENVELOPE_PROJECT_ROOT` — were uncovered. Floor 4 fires before the coverage loop,
which is the correct order: each floor names the filter it replaces, so re-introducing
either filter turns the pin red rather than green.

## Task 2 — the RED output, verbatim

`rtk proxy cargo test --test envelope_wrapper_class`, against the unfixed tree:

```
failures:
    a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position
    a_wrapped_forge_command_quoting_a_governed_name_is_refused_and_the_unwrapped_one_is_not
    the_decoy_rule_discriminates_rather_than_blanket_denying_the_shape
    the_disclosed_costs_of_the_command_position_rule_are_pinned_beside_the_spellings_that_work

test result: FAILED. 11 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
```

The property's own first failing case, printed as designed:

```
thread 'a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position'
panicked at tests/envelope_wrapper_class.rs:1283:17:
assertion `left == right` failed:

A GOVERNED PROGRAM IN A WRAPPER'S OPERAND SLOT WAS PERMITTED.

  command : bash -lc "FOO=bar env -u git 'nice' -n 10 git push --force origin main"
  base    : git push --force origin main
  recipe  : prefix="FOO=bar " layer=BashDashLC depth=1 decoy="env -u git" at=0
  got     : exit 0 reason permit(the guard answered nothing)
  seed    : 0x1912c0de5eed0060
```

That case is the finding in one line: an outer `bash -lc` payload, an assignment prefix,
a decoy spliced **outside** an ordinary wrapper — a shape the 19-12 alphabet could draw
every part of except the four characters `-u git`, and therefore a shape 1680 generated
cases certified without being able to fail on.

**The eleven pre-existing tests in the file passed UNMODIFIED in the RED run and in the
green run**: the invariance property, the permitted corpus, the alphabet-absence control
with its positive controls, the `T-19-75` discrimination pair, the three generalisation
rows, the two `T-19-74` bound tests, `the_residual_begins_exactly_at_the_command_line_boundary`
and the doc-disclosure pin. No existing test in that file was edited.

**The decoy corpus as generated after the fix:** 720 cases, 10 decoy operands, every
operand used, and the both-sides-wrapped floor satisfied.

## Fail-first evidence for the two second layers

Both arms are unreachable through `guard_in` once the resolver refuses first, so each is
pinned by a unit test on the pure function and each was proved fail-first by reverting
the arm, running, observing red, and restoring.

**`classify_git`'s governed-verb refusal** — arm deleted, `--lib envelope::policy::tests::classify_git_refuses`:

```
thread 'envelope::policy::tests::classify_git_refuses_an_argv_whose_own_verb_is_a_governed_program'
panicked at src/envelope/policy.rs:3030:13:
`git ["git", "push", "--force"]` puts a governed program in its own VERB slot, which no
valid invocation does. Reaching the denylist's default arm and answering `Allow` here is
exactly how a mis-index became a permit (`T-19-60`, audit 2).

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1131 filtered out
```

**`forge_subcommand_names_a_governed_program`** — body replaced with `None`,
`--lib envelope::policy::tests::the_forge_predicate`:

```
thread 'envelope::policy::tests::the_forge_predicate_names_a_governed_first_subcommand_word'
panicked at src/envelope/policy.rs:3081:9:
assertion `left == right` failed
  left: None
 right: Some("gh")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1131 filtered out
```

Both restored immediately; `--lib envelope::policy` green at 61 passed afterwards.

**The measurement the plan asked for, confirmed:** the auditor's named one-function fix
is genuinely insufficient on its own. `env -u git timeout 5 git push --force origin main`
reaches `classify_git` with the verb `timeout`, and the forge decoy never enters
`classify_git` at all. Neither arm is dead code and neither may be deleted as
unreachable — they are the layer that holds if resolution mis-indexes a third time.

## The four `T-19-86` exit codes, measured

Measured through `hooks::guard_in` with one fresh `TempDir` per row, **before** the fix
(commit `b5a211b`, all four green) and **after** it (commit `7ef7ba2`, all four still
green — by design):

```
exit=0  git submodule foreach git push --force origin main
exit=0  git rebase -x "git push --force origin main" HEAD~3
exit=0  git bisect run sh -c "git push --force origin main"
exit=0  git -c alias.p='!git push --force origin main' p
```

They resolve at the **head** — correctly, because the head is the command position — and
are then permitted by `classify_git`'s denylist default arm, whose verbs here are
`submodule`, `rebase`, `bisect` and (after `scan_leading` consumes `-c alias.p=…`) `p`.
Green before and after is the intended shape: the pin reads the head shortcut's cost from
the other side.

## Pinned permits this plan's disclosed over-refusal converted to refusals

**One**, and it is a row this plan wrote in its own RED commit rather than a pre-existing
assertion belonging to an earlier round:

| Row | Old verdict | New verdict | Justification |
|---|---|---|---|
| `SSH_AUTH_SOCK=/tmp/evil git fetch origin` (`tests/envelope_command_position.rs`) | PERMITTED (`the_enabling_half_of_t_19_82_is_permitted_today`) | REFUSED under `hook_bypass_blocked` (`reassigning_a_removed_envelope_key_is_refused`) | This is `T-19-82`'s enabling half and the flip is the fix. `SSH_AUTH_SOCK` is one of `cred::build_env_in`'s two REMOVAL entries — the belt D-16 relies on — and it was uncovered because the drift pin filtered removals out with `value.is_some()`. The plan's Task 1 required the row asserted PERMITTED against the unfixed tree precisely so the key-set completion is measured as a **change** rather than asserted about; Task 3 flips it. |

**No pre-existing test from 19-11 or 19-12 was edited.** In particular
`the_t_19_74_residual_is_permitted_and_the_cost_of_closing_it_is_measured_beside_it`,
`the_bounds_of_the_t_19_74_residual_are_refused_which_is_what_makes_it_narrow`,
`the_residual_begins_exactly_at_the_command_line_boundary`,
`resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`,
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`, the invariance
property and the permitted corpus are all green and unmodified. `X=git; env $X push
--force origin main` is still permitted, and its previously unpinned decoy form
`env -u git $X push --force origin main` is now pinned permitted too, with a comment
naming it an accepted residual rather than a desirable behaviour.

## Disclosed costs, each pinned beside the spelling that still works

| Now refused | Still permitted | Where |
|---|---|---|
| `sudo -u git git status` | `git status`, `sudo -E git status` | `envelope_wrapper_class.rs` |
| `env -u gh gh pr list --limit 5` | `gh pr list --limit 5`, `env gh pr list --limit 5` | `envelope_wrapper_class.rs` |
| `timeout $T git fetch origin` | `git fetch origin`, `timeout 60 git fetch origin` | `envelope_wrapper_class.rs` |
| `env gh pr create --title "git push --force"` | `gh pr create --title "git push --force"` | `envelope_wrapper_class.rs` |
| `FOO=GIT_ASKPASS echo hi` | `FOO=bar echo hi`, `echo FOO=GIT_ASKPASS` | `envelope_command_position.rs` |

All confined to driven runs, all legible when they fire, and all **discriminating** rather
than blanket: the head shortcut leaves every unwrapped command untouched, so commit
messages, PR titles and `env -u git ls` still work.

## Deviations from plan

### 1. `${K}_COMMAND` is not closed by the expansion-prefix rule — a finding, now `T-19-87`

The plan's must-have truth and the plan-check both asserted that
`K=GIT_SSH; env -u ${K}_COMMAND git fetch origin` is closed by the expansion-prefix rule,
reasoning that `${K}_COMMAND` carries `Token.expansion`. **Measured, it does not.**
`split_words` treats `{` and `}` as **separators** (`policy.rs`'s `SEPARATORS`, and the
tokenizer arm beside `;` and `|`), so the word never survives as one token. The line
fragments into three segments —

```
env -u $   |   K   |   _COMMAND git fetch origin
```

— and the guard judges each on its own. The third resolves `git` behind an ungoverned head
with exactly one candidate and no expansion between them, so it is `git fetch`, which is
allowed. The row stayed green after the fix; that is how it was caught.

**What was done:**

- The row's spelling was changed to `env -u $K git fetch origin` — the same class-level
  claim (an envelope key removed through a name spelled nowhere in the command line), in a
  spelling the splitter preserves. It is genuinely RED before the fix and green after, and
  it is what `the_indirect_spelling_no_literal_match_can_find_is_refused_too` now pins.
- The brace form is pinned at its current **PERMITTED** verdict in
  `the_brace_expansion_spelling_is_a_residual_this_plan_does_not_close`, with the bound
  asserted beside it: `env -u ${K}_COMMAND git push --force origin main` is still refused
  under `force_push_blocked`, because the segment carrying the command still resolves it.
  What the fragmentation hides is the **env-key removal**, which is exactly the `T-19-81`
  harm.
- Registered as **`T-19-87`** in `deferred-items.md`. Not fixed: closing it means changing
  what `{`/`}` do in `split_words`, which is real shell grouping syntax (`{ cmd; }`) and a
  blast radius across every command the guard splits — far outside a plan scoped to four
  items.
- It is **not** added to `19-SECURITY.md`. The plan permits exactly one appended
  subsection carrying the `T-19-86` row, and an execution-time finding has a third
  provenance again; blurring it into a planning-time registration would be the same
  forgery the prohibition exists to prevent.

### 2. Two doc comments outside `#[cfg(test)]` in the RED commit

Task 1's `<done>` says its only `src/` hunks are inside `#[cfg(test)] mod tests`, while its
`<action>` requires updating `ENVELOPE_ENV_KEYS`' doc and the pin's doc so neither keeps
claiming the narrower coverage. The action was followed. Both hunks are doc comments and
carry no logic; `production_code` in the alphabet-absence control strips `///` lines, so
nothing mechanical depends on them.

### 3. `env --unset=git` was not used as a decoy operand

An early draft of `DECOY_OPERANDS` included the `=`-attached option form. It is **not** a
decoy: `--unset=git` starts with `-` and is skipped by the candidate scan, leaving one
candidate and an ordinary resolution. Replaced with `sudo --user git`, the separate-value
long-option form, which does put a bare governed token in a non-head position. Ten entries
shipped, over the floor of eight.

## What remains uncovered

### 1. `T-19-86` — a governed program's own operand naming a governed command — OPEN

**`T-19-60` is closed for the WRAPPER-OPERAND sub-class only.** The sub-class closed here
is: a token that is not the effective program captures the resolver's index because it is
spelled `git`/`gh`/`glab` in a **wrapper's** operand slot. An adjacent member of the same
class is left open: a **governed** program's own operand naming a governed command
(`git submodule foreach git push --force origin main` and its three siblings above).
Those resolve at the head — correctly — reach `classify_git`'s denylist default arm, and
are permitted before this plan and after it.

They were found while planning this round. A plan cannot both discover a threat and be the
plan that measured it fail first; narrowing the closure claim was the correct response and
closing them inside this plan was not. They are pinned at their current permitted verdict,
disclosed in `resolve_program`'s doc beside `T-19-74` and `T-19-75`, and registered in
`19-SECURITY.md` (one appended, clearly-attributed planning-time subsection — the audit
tables, trail, risk log and sign-off are untouched) and in `deferred-items.md`.

**`/gsd-secure-phase 19` is NOT cleared by this plan alone.**

### 2. `T-19-87` — `${VAR}` fragments a command past the splitter — OPEN

New, found during execution of this plan, described in full under *Deviations* above.
Pinned permitted, bounded on the side where it does not help an attacker, registered in
`deferred-items.md`, not fixed. Same suggested owner as `T-19-86`: both are
splitter/classifier questions rather than resolver questions.

### 3. `T-19-74` (AR-19-10) — unchanged and deliberately so

`env $X push --force origin main`, `X=git; env $X push --force origin main` and the decoy
form `env -u git $X push --force origin main` all stay **permitted**. The new
leading-assignment value check covers envelope **keys** only; step 7's `GOVERNED_PROGRAMS`
value check keeps its existing position after resolution has failed, and the asymmetry is
documented in the code with its reason rather than tidied away.
`the_residual_begins_exactly_at_the_command_line_boundary` is green and unmodified.

### 4. `T-19-84` and `T-19-85` — out of scope, unchanged

Not taken on, by the explicit user scoping decision. Note that `T-19-84` is *about* the
truthfulness of `resolve_program`'s `T-19-74` paragraph, and this plan rewrote step 5's
doc bullet and added a third residual bullet without touching that paragraph's claims —
so `T-19-84` is neither closed nor worsened.

### 5. `T-19-61` … `T-19-73` — out of scope, unchanged

Open and unaccepted by explicit user decision. `deny`'s `?`-before-`park_refusal` ordering
(`T-19-61`) is untouched even though `hooks.rs` was edited; no park-coverage control was
added; `cred.rs`'s env scrub, `advisory.rs`, the honesty statement and the second-carrier
table are untouched.

## Known stubs

None. Every control this plan added is wired to production behaviour and was observed
failing against it first.

## Self-Check: PASSED

Files claimed created/modified, verified on disk:

```
FOUND: tests/envelope_command_position.rs
FOUND: tests/envelope_wrapper_class.rs
FOUND: src/envelope/policy.rs
FOUND: src/envelope/hooks.rs
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
```

Commits claimed, verified in `git log`:

```
FOUND: b5a211b  test(19-13): RED — the command-position reproducers and the widened drift pin
FOUND: 2e1b799  test(19-13): RED — the governed-operand alphabet the 19-12 corpus could not generate
FOUND: 7ef7ba2  feat(19-13): establish command position structurally, and register what it leaves open
```

`git diff a41e431..HEAD --numstat` touches exactly the six files in `files_modified` and
nothing else; `Cargo.toml` and `Cargo.lock` are absent.



