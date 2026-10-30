---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 23
subsystem: envelope
tags: [security, config-resolution, T-19-103, T-19-104, T-19-105, T-19-106, T-19-107]
status: complete

requires:
  - "19-22 (the corpus, RED — the 1673 baseline and the nine RED names)"
  - "19-SECURITY.md audit 7 (T-19-103 … T-19-107)"
provides:
  - "the CONFINEMENT clause — an unbounded config assignment is unresolvable"
  - "GIT_CONFIG_PARAMETERS on ENVELOPE_ENV_KEYS, plus a SECOND drift-pin source"
  - "CONFIG_VALUE_OPTS and GIT_GLOBAL_SELF_CONTAINED_OPTS corrected and pinned"
  - "the spawn compensating control's two gaps closed"
affects:
  - "/gsd-secure-phase 19, which this plan does NOT clear"

tech-stack:
  added: []
  patterns:
    - "a rule as a PROPERTY of the key rather than a longer list of keys"
    - "a second pin SOURCE for a class the first pin's source structurally cannot see"
    - "halt-and-report on an unsatisfiable evidence row rather than editing the assertion"

key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/envelope/cred.rs
    - tests/envelope_config_resolution.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md

decisions:
  - "CARRIER BEFORE VERB — an unbounded config assignment makes every downstream classification a statement about a command the guard cannot see, so the refusal names the carrier"
  - "The rule reads the SECTION and deliberately reads neither the subsection nor the variable, covering both open families by construction"
  - "The residue fails OPEN and is NOT handed to the pin — no automated control covers a future third indirection section"
  - "GIT_CONFIG_NOSYSTEM listed as a measured DEFEAT with an INERT harm, never called a bypass"
  - "T-19-17r left OUTSTANDING with no AR-19-13 and no acceptance made"

metrics:
  duration: "~2h"
  completed: 2026-09-03

actuals:
  tokens: 149000
  tasks: 4
  commits: 5
---

# Phase 19 Plan 23: The Rules for git's Config Resolution — Summary

The guard stops asking whether an assignment spells `core.hooksPath` and starts
asking whether it can establish what `core.hooksPath` will be.

## FIRST: `/gsd-secure-phase 19` is NOT cleared by this plan

**`T-19-86` and `T-19-91` both remain OPEN at `high`.** `T-19-96`, `T-19-74`,
`T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` remain open and unaccepted by
explicit user decision.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" appears anywhere in this plan's output.

Closing `T-19-103` is a **RESTORATION of layer 3's catch** of `T-19-86`'s
persisted-alias arm — never a closure of `T-19-86`.

## A BLOCKER was hit, reported rather than worked around

This is recorded before the results because it is the round's most important
event, and because the round already applies this discipline to its own
fail-open admission's corrected draft.

After the confinement clause landed, `19-22`'s
`the_force_push_compositions_are_already_refused_and_are_controls_not_reproducers`
turned red. It asserted `force_push_blocked` for five `-c include.path=`
`--force` compositions. That row and section 7's ordering pin —
`git -c include.path=$F -c core.hooksPath=/dev/null push --force origin main` at
`envelope_assertion_failed` — **differ only in tokens AFTER the first unbounded
assignment, which `scan_leading` never reads because it returns at the first
assignment it cannot bound.** No rule raised inside the one left-to-right scan
produces different identifiers for them, and a rule that could would have to read
past the first unbounded assignment — which the ordering pin exists to forbid.
**The two assertions were unsatisfiable together.**

**The executor halted and reported rather than editing an evidence row**, which
this plan's prohibitions forbid, and rather than deciding a security-relevant
semantic that three audits and a plan-check had reserved.

The defect is `19-22`'s. The demotion of those five rows to verdict-only controls
was stated in **four** places and implemented in none:

- the control's own comment — *"Their verdict does not move; only their IDENTIFIER
  may, and section 9 records that separately without asserting it"*;
- `19-22-SUMMARY.md` §"The two rows RECORDED rather than asserted", which lists
  this exact command;
- `19-22-PLAN-CHECK.md` Check 4 (*"the five `--force` compositions are correctly
  demoted to controls"*) and Check 5 (*"No replacement exception needed or
  granted"*);
- `19-23-PLAN.md:668`, predicting the identifier moves to
  `envelope_assertion_failed`.

**The orchestrator authorized the correction** as a named, narrow exception scoped
to the identifier constant on those five rows, with option 3's recording
discipline. It was re-verified against the built binary first (all five uniform at
exit 2 `envelope_assertion_failed`, walks empty), committed **FIRST and on its
own**, RED at base with zero `src/` hunks. `tests/envelope_wrapper_class.rs` was
not touched and no other assertion was weakened.

**CARRIER BEFORE VERB was decided explicitly rather than by an edit.** An
unbounded config assignment means the guard cannot establish what configuration
the command will run under, so every downstream classification —
`force_push_blocked` included — describes a command whose behaviour the guard
cannot bound. Refusing at the carrier and saying so is the honest verdict;
reporting `force_push_blocked` would name a specific hazard while the guard is in
fact unable to see the command at all (D-24). That reasoning is written into the
rule's doc, the corrected control, both derived-row pins and the record.

## The commits

| SHA | What |
|---|---|
| `a8b8632` | `test(19-23)`: correct a corpus defect — the five `--force` compositions assert the identifier they were demoted from (**zero `src/` hunks, RED at base**) |
| `2313ecf` | `feat(19-23)`: `T-19-103` — an unbounded config assignment is unresolvable |
| `2448064` | `feat(19-23)`: `T-19-104` — `GIT_CONFIG_PARAMETERS` on the list, and a SECOND pin source |
| `fe0d6c7` | `feat(19-23)`: `T-19-106`, `T-19-107`, the spawn control's two gaps, and the two derived rows |
| `2190cf5` | `docs(19-23)`: the round-8 closure record |

## `19-22`'s RED set, confirmed STILL RED before any production line moved

All nine names re-run against `b9d8eca`. **None was already green.**

```
tests/envelope_config_resolution.rs
  test result: FAILED. 21 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
tests/envelope_wrapper_class.rs
  test result: FAILED. 38 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

```
AN ENVIRONMENT CARRIER OF CONFIGURATION WAS PERMITTED.
  command : "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\" git status"
  base    : git status
  splice  : AssignmentPrefix(GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'")
  got     : exit 0 reason permit(the guard answered nothing)
  left: 0
 right: 2
```

```
`git -c include.path=/tmp/evil.cfg -c core.hooksPath=/dev/null push --force origin main`
must be refused UNDER `envelope_assertion_failed`. Asserting the identifier and not
merely the exit code is what stops a row from passing because it was refused for an
unrelated cause (D-24).
Got: gsd-meta-manager envelope: REFUSED (reason: hook_bypass_blocked) — …
```

```
`git -c include.path=/tmp/evil.cfg commit -m x` must be REFUSED.
  stdout:   stderr:
Walked listing:
(the envelope root is empty)
  left: 0
 right: 2
```

All nine are now **GREEN and named**:

`tests/envelope_config_resolution.rs` —
`after_19_23_a_config_indirection_carrier_is_refused_on_a_permitted_base`,
`after_19_23_a_config_indirection_carrier_is_refused_on_the_in_namespace_push`,
`after_19_23_the_environment_carrier_of_configuration_is_refused`,
`after_19_23_the_disclosed_bare_word_cost_of_the_new_list_entry_is_refused`,
`after_19_23_the_disclosed_cost_of_reading_the_section_and_not_the_variable_is_refused`,
`after_19_23_the_scan_order_decides_which_clause_names_a_line_carrying_both`,
`after_19_23_the_composition_rows_are_refused_which_proves_rounds_6_and_7_load_bearing`.

`tests/envelope_wrapper_class.rs` —
`a_config_indirection_the_guard_cannot_bound_fails_closed_on_every_base`,
`an_environment_carrier_of_configuration_fails_closed_on_every_base`.

## Every RED row, before and after

| Row (abbreviated) | before | after | walk |
|---|---|---|---|
| `git -c include.path=$F commit -m x` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=$F status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=$F push origin refs/heads/gsd-auto/alpha/w` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c includeIf.gitdir:/tmp/.path=$F status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git --config-env=include.path=EVILVAR status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git --config-env include.path=EVILVAR status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c INCLUDE.PATH=$F push origin refs/heads/…` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.pathx=$F status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=$F -c core.hooksPath=/dev/null push --force …` | 2 `hook_bypass_blocked` | 2 `envelope_assertion_failed` | EMPTY |
| the four composition rows (round 6, round 7, wrapper, segment) | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `GIT_CONFIG_PARAMETERS=… git commit -m x` | 0 permit | 2 `hook_bypass_blocked` | EMPTY |
| `GIT_CONFIG_PARAMETERS=… git push origin refs/heads/…` (3 spellings) | 0 permit | 2 `hook_bypass_blocked` | EMPTY |
| `echo GIT_CONFIG_PARAMETERS` | 0 permit | 2 `hook_bypass_blocked` | EMPTY |

The ordering pin's other direction —
`git -c core.hooksPath=/dev/null -c include.path=$F push --force origin main` —
stays at `hook_bypass_blocked`. **Both directions green is what proves the refusal
is raised at the first unbounded assignment inside the one scan rather than in a
second pass.**

## The two rows `19-22` could not derive — MEASURED and pinned with their clauses

| Row | before | after | the clause that produced it |
|---|---|---|---|
| `GIT_CONFIG_NOSYSTEM=1 git push origin refs/heads/gsd-auto/alpha/w` | 0 permit | 2 `hook_bypass_blocked` | Task 2's `ENVELOPE_ENV_KEYS` entry — `tampers_with_envelope_env` now covers the assignment prefix. `T-19-104`'s class, and the identifier says so |
| `git -c include.path=$F push --force origin main` | 2 `force_push_blocked` | 2 `envelope_assertion_failed` | Task 1's confinement clause, raised at the first unbounded assignment before any verb is classified — verdict preserved, identifier moved |

**Neither became PERMITTED.** Had either done so it would have been a finding
about the rules and not a row to pin.

## The gate

```
rtk proxy cargo test --no-fail-fast          exit 0
passed + failed = 1680   (1680 passed, 0 failed, 13 ignored, 43 result lines)
19-22's recorded total 1673 + 7 NEW #[test] fns = 1680     identity holds, no residual
```

A red test RAN, so red→green leaves the total unchanged and every increase comes
ONLY from new fns. The seven, counted from `git diff b9d8eca..HEAD`: five in
`policy.rs`'s own test module
(`every_indirection_section_the_guard_names_really_outranks_the_envelopes_own_injection`,
`the_section_helper_reads_the_section_and_neither_the_subsection_nor_the_variable`,
`every_key_that_defeats_the_envelope_is_covered_and_its_defeat_is_measured`,
`every_config_value_opt_really_takes_a_separate_value_on_the_installed_git`,
`the_four_structural_arms_of_leading_git_option_are_pinned_against_the_installed_git`)
and two in `tests/envelope_config_resolution.rs`. **Zero `#[test]` fns removed.**

`failed` is **0** — none of the three documented flakes fired, and their absence
is not evidence they are fixed. Counts read with `rtk proxy grep` over a
redirected log (D-34).

`cargo build` and `cargo clippy -- -D warnings` both exit 0. `cargo clippy --tests`
is NOT the gate: it fails at base on four pre-existing lints in `src/browser.rs`
and `src/project_creator.rs`, which were not touched.

### All FOURTEEN `envelope_*` binaries RAN

| Binary | passed | failed | ignored |
|---|---|---|---|
| `envelope_advisory` | 10 | 0 | 0 |
| `envelope_argv_deletion` | 20 | 0 | 0 |
| `envelope_callee_grammar` | 19 | 0 | 0 |
| `envelope_command_position` | 18 | 0 | 0 |
| **`envelope_config_resolution`** | **30** | 0 | 0 |
| `envelope_credential` | 6 | 0 | 0 |
| `envelope_expansion_slots` | 32 | 0 | 0 |
| `envelope_hook_refusals` | 7 | 0 | 0 |
| `envelope_literal_decision` | 43 | 0 | 0 |
| `envelope_pr_cap` | 11 | 0 | 0 |
| `envelope_tracer` | 6 | 0 | 0 |
| `envelope_wiring` | 14 | 0 | 0 |
| `envelope_wrapper_bypass` | 13 | 0 | 0 |
| **`envelope_wrapper_class`** | **40** | 0 | 0 |

Fourteen, not thirteen. `envelope_expansion_slots`, `envelope_command_position`,
`envelope_literal_decision`, `envelope_argv_deletion` and
`envelope_callee_grammar` are fully GREEN and **unmodified**, so rounds 4, 5, 6
and 7's evidence was not disturbed.

## The design question, answered, with the three rejected options costed

**(i) Add `include.path` to `is_hooks_path_key`** — REJECTED.
`includeIf.<arbitrary condition>.path` is an open family, so the list is wrong the
moment a condition type is used; and it would attribute the refusal to
`HookBypassBlocked` on a line where the guard established no hooks-path write,
which is D-24's own prohibition.

**(ii) Read the included file and resolve the config at guard time** — REJECTED on
three independent measured grounds: the guard runs synchronously on the
`PreToolUse` critical path where a reproduced 180-240 second hang is why
`push_needs_resolved_dests` exists; the file may not exist at guard time or may
change between guard and exec (TOCTOU); and `includeIf`'s conditions depend on the
repository the command will run in, which a pure argv function does not know.

**(iii) Refuse every `-c`** — REJECTED and pinned red by the permitted half.

**(iv) ADOPTED — read the SECTION and read nothing else.** git's config graph is
spliced from elsewhere by exactly one mechanism, identified by the section half of
the key. Reading the section and deliberately reading neither the subsection nor
the variable covers **both** open families by construction.

## The residue, stated plainly and NOT handed to any control

**This is NOT a fifth inversion.** Rounds 3, 5, 6 and 7 each made the guard's
silence a refusal. **This rule cannot, and its SILENCE IS A PERMIT**, because the
complement is unbounded: a fail-closed default over config sections would have to
refuse every section the guard has not enumerated, and users legitimately set
arbitrary ones (AR-19-11).

> **A future git that adds a THIRD indirection section is not covered, this rule
> fails OPEN on it, and there is NO automated control over that direction.** The
> pin holds the REVERSE direction — it turns red if the installed git stops
> honouring a section the constant already names — and **it cannot observe a
> section it does not name, because it iterates the entries and an entry that does
> not exist is never probed.** A third indirection section reaches
> `core.hooksPath` silently until a human adds it.

That whole sentence, including its second half, is in `INDIRECTION_SECTIONS`'s own
doc, in the pin's doc and in the record.

**PROVENANCE.** An earlier draft of this plan attributed the residue to the
real-git pin — *"the only control over that direction is the pin"* — and a
plan-check caught it as **`T-19-107`'s own failure mode, false reassurance in a
control's own doc, inside the round that registers `T-19-107`**. A round that made
and corrected that mistake should say so rather than present the corrected text as
the only text there ever was.

## Case-insensitivity, measured in BOTH halves — and no existing pin changed

Re-measured this round against `git version 2.43.0`, with the exact triplet
`cred::hooks_path_env` emits as the control:

```
control:  GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS   -> /ENV_WINS
+ -c include.path=<file>                                              -> /INCLUDE_WINS
+ -c INCLUDE.PATH=<file>                                              -> /INCLUDE_WINS   <- CASE
+ -c includeIf.gitdir:<p>.path=<file>                                 -> /INCLUDE_WINS
+ -c INCLUDEIF.gitdir:<p>.PATH=<file>                                 -> /INCLUDE_WINS   <- CASE
+ --config-env=include.path=EVILVAR                                   -> /INCLUDE_WINS
+ --config-env include.path=EVILVAR                                   -> /INCLUDE_WINS
+ GIT_CONFIG_PARAMETERS="'core.hooksPath=/PARAM_WINS'"                -> /PARAM_WINS
+ GIT_CONFIG_PARAMETERS="'include.path=<file>'"                       -> /PARAM_INCLUDE_WINS
+ -c include.pathx=<file>                                             -> /ENV_WINS  (git IGNORES it)
+ -c notinclude.path=<file>                                           -> /ENV_WINS
+ -c user.name=x / -c core.pager=cat                                  -> /ENV_WINS
persisted `git config include.path <file>`, alone                     -> /INCLUDE_WINS
persisted `git config include.path <file>`, under injection           -> /ENV_WINS  (INERT)
git -c a=b version         -> git version 2.43.0, rc 0
git -c a=b config --get a  -> error: key does not contain a section: a, rc 1
```

Git folds the SECTION and the VARIABLE and leaves the SUBSECTION case-sensitive,
so the clause compares the section `eq_ignore_ascii_case`. **No existing pin
changed**: `is_hooks_path_key` was ALREADY `eq_ignore_ascii_case("core.hookspath")`
and was not touched; `git -c CORE.HOOKSPATH=/dev/null status` is still
`hook_bypass_blocked`. One doc sentence records that the guard's whole-key fold is
MORE permissive than git for a subsectioned key — the safe direction.

The persisted-`git config` row is why the `git config` subcommand is **not** a
second escape and correctly needs no clause.

## The class reproduced END TO END — re-measured, not cited

A claim in this codebase about which forms outrank the injection was already known
false, so this round reproduced the behaviour rather than repeating `19-22`'s
SHAs. Fixture rebuilt, hook delivered exactly as the envelope delivers it, remote
SHA recorded before and after each leg:

```
leg 1  plain in-namespace push        rc=1, hook REFUSED   70af3d7 -> 70af3d7  UNMOVED
leg 2  -c include.path=<evil>         rc=0                 70af3d7 -> cea98c6  MOVED
leg 3  GIT_CONFIG_PARAMETERS carrier  rc=0                 cea98c6 -> 18102fc  MOVED
leg 4  pre-commit: control rc=1 (refused), carrier rc=0     18102fc -> a941712  MOVED
```

**All four legs reproduced independently. None failed.**

## The SECOND PIN SOURCE, with the structural reason it exists

`ENVELOPE_ENV_KEYS` has been wrong FOUR times — `GIT_SSH_COMMAND` (19-11),
`SSH_AUTH_SOCK`/`SSH_AGENT_PID` (`T-19-82`), `GSD_MM_RUN_ID` (`T-19-90`) and
`GIT_CONFIG_PARAMETERS` (`T-19-104`). Its existing drift pin caught the first
three and **structurally cannot see the fourth**: its source is
`cred::EnvelopeEnv::with_run_id(build_env_in(…))` — the keys the envelope SETS or
REMOVES — and a key the envelope neither sets nor removes is not in its source at
all. **The fix is a second SOURCE, not a wider filter.**

`ENVELOPE_ENV_DEFEATING_KEYS` carries each defeating key WITH the envelope key it
defeats, in the DATA:

| Defeating key | Defeats | Measured |
|---|---|---|
| `GIT_CONFIG_PARAMETERS` | the `GIT_CONFIG_COUNT` triplet | control `/ENV_WINS` → `/PARAM_WINS` |
| `GIT_CONFIG_NOSYSTEM` | `GIT_CONFIG_SYSTEM` | `credential.helper` reads `evil`; with the key set, rc 1, nothing read |

**`GIT_CONFIG_NOSYSTEM`'s harm is INERT and it is NEVER called a bypass.**
`cred::write_gitconfig` points BOTH `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` at
the same helper-free file, so suppressing the system read removes a deny the
global pointer duplicates. **That duplication is now itself under test** — the pin
asserts the two variables carry the SAME path and that the file they name still
resolves no credential helper with the system read suppressed — so a later change
cannot spend the inertness silently.

The pin asserts every defeating key covered in both spellings, every defeated key
covered, MEASURES each defeat against real git with the envelope's own mechanism
as the control, and carries a non-vacuity negative control. It does not skip when
git is absent.

## The `cred.rs` correction — A NAMED, NARROW, DELIBERATE EXCEPTION

The doc at `cred.rs:241-248` claimed:

> *the one form that outranks this injection is `git -c core.hooksPath=… push` —
> which plan 19-02 denies by name at the tool boundary for exactly that reason.*

**Measured, there are FIVE and four were permitted until this round:**

| Form | Resolves to | Closed by |
|---|---|---|
| `git -c core.hooksPath=… push` | command-line precedence | plan 19-02, by name |
| `git -c include.path=<file>` | `/INCLUDE_WINS` | 19-23's confinement clause |
| `git -c includeIf.<cond>.path=<file>` | `/INCLUDE_WINS` | the same clause, by SECTION |
| `git --config-env=include.path=<VAR>` | `/INCLUDE_WINS` | the same clause, second carrier |
| `GIT_CONFIG_PARAMETERS="'core.hooksPath=…'"` | `/PARAM_WINS` | 19-23's `ENVELOPE_ENV_KEYS` entry |

The server-side-branch-protection sentence is KEPT, and a sentence was added that
the closures are client-side and the confinement clause fails OPEN on a future
third indirection section.

**This is the ONLY line of `cred.rs` this round opened.** `git diff --unified=0`
shows ONE hunk at `:244-248` and **zero non-doc lines changed** in the file.
**`T-19-61` … `T-19-73` were NOT taken on.** Leaving a claim the round's own
evidence contradicts would have been the `T-19-84` failure mode, which is why the
exception was granted — and it is recorded as an exception rather than presented
as ordinary scope.

## The over-refusal cost, measured from BOTH sides

**The one row this round's own rule moves permitted → refused** is
`git -c include.pathx=/tmp/evil.cfg status`: a key real git IGNORES, refused
because the rule reads the SECTION and deliberately not the variable. Safe
direction, named here rather than left for audit 8. The correct response to it is
NOT to start reading the variable.

Measured at exit 0 after the rules, all walks EMPTY:
`git -c user.name="$NAME" commit -m x`, `git -c core.pager=cat log`,
`git -c a=b status`, `git -c includepath=/tmp/evil.cfg status`,
`git -c notinclude.path=/tmp/evil.cfg status`, `git --git-dir=/tmp/g status`,
`git -C /tmp status`, `git --no-pager status`.

`git -c a=b push --force origin main` and
`git -c a=b --attr-source HEAD push --force origin main` stay at
`force_push_blocked`. `git -c CORE.HOOKSPATH=/dev/null status` stays at
`hook_bypass_blocked`. `echo GIT_CONFIG_PARAM` stays permitted beside
`echo GIT_CONFIG_PARAMETERS` refused.

**`git -c a=b status` is the DOTLESS fence** — refusing it would have turned round
7's entire callee-grammar generative property permanently red behind
`CALLEE_KNOWN_LEADING_PREFIX`.

## `T-19-107` — the correction, recorded BESIDE plan 19-21's record

`19-21-SUMMARY.md` and the appended plan-19-21 record are **NOT edited**.

The claim *"ZERO over-refusal cost on git 2.43.0"* was **false by one measured
row**. `git -v` is accepted by this git — `git -v` prints `git version 2.43.0` and
`git -v XVALUE version` prints it too, terminating exactly as `--version` does —
it was in NEITHER constant, NOT in the disclosed UNPROBED set, and measured at
exit 2 `envelope_assertion_failed` beside `git --version` at exit 0.

**What is true instead**: the cost was ONE measured row and that row is removed;
the remaining commands moving permitted → refused are ones git ITSELF rejects.
`-v` now sits in `GIT_GLOBAL_SELF_CONTAINED_OPTS` where the `>= 8` floor and the
two-sided probe cover it, and the probe classified it before it was written there.
`git -v` and `git -v status` are now exit 0.

## `T-19-106`'s two halves

**(a) `CONFIG_VALUE_OPTS` LOST `--comment`**, and the removal is part of the fix:

| Command | before | after |
|---|---|---|
| `git config --comment core.hooksPath /dev/null` | **0** (`/dev/null` read as the key) | 2 `hook_bypass_blocked` |
| `git config core.hooksPath /dev/null` | 2 `hook_bypass_blocked` | 2 `hook_bypass_blocked` |

Real git answers ``error: unknown option `comment'``. That is the identical
over-consuming mis-index `19-21` removed `--super-prefix` over, in a second
constant, inert only because git rejects the option. A new two-sided pin covers
every remaining entry and **reads git's own classification rather than inferring
one**: `git config <opt>` with no value answers ``option `X' requires a value``,
``switch `X' requires a value``, or ``unknown option `X'``. **No per-entry variant
value is needed at all** — the round-7 pins need variants because their probe must
REACH a verb; this one does not — and that is stated rather than left implicit.
Two negative controls: `--bogus-config-opt` and `--list`.

**(b) The four STRUCTURAL arms of `leading_git_option` are pinned**, and two
asserted a grammar the installed git contradicts:

| Arm | Stated premise | Measured |
|---|---|---|
| arm 3, `-c<rest>` | *"git's short-option parser accepts `-ckey=value` with no space"* | `git -cuser.name=x version` → `unknown option: -cuser.name=x` |
| deviation-1's arm 4, `-C/tmp` | `-C` with an attached value | `git -C/tmp version` → `unknown option: -C/tmp` |

Both **inert in the SAFE direction**, **neither arm's behaviour changed**. Docs
corrected to state what git does, why the arms are kept (arm 4's removal regressed
`git -C/tmp push --force origin main` and broke a `19-20` pin), and that arm 4's
`GIT_GLOBAL_VALUE_OPTS.contains(&head)` check is a **ONE-ELEMENT test today**. The
bundle half (`git -pc user.name=x version` → `unknown option: -pc`) is pinned too.

## The spawn compensating control's two repairs, with their measured line numbers

**(a) The marker set was NARROWER than the control it replaces** —
`["Command::new(", "process_group("]` against `tests/spawn_seam_guard.rs:108`'s
three. **`CommandWrap::with_new(` is a spelling this repository actually uses, at
`src/executor/claude.rs:480`**, so a production spawn written that way in
`policy.rs` would have been invisible to BOTH controls. Added, with the
self-invalidation hazard named: the marker constant lives inside `#[cfg(test)] mod
tests` and **must not be hoisted above the sentinel**. The bare `contains` is kept
with the reason stated — `calls_marker` exists to avoid FALSE POSITIVES, and this
control asserts an ABSENCE, so a false positive here fails CLOSED.

**(b) The truncation guard was thin at exactly the seam that failed this round.**
Measured at `b9d8eca`: `fn scan_leading` at line **374**, 40,000 raw bytes reached
at line **778**, of a production half running to line **4,262** and **207,358
bytes**, one `#[cfg(test)]` sentinel at 4,263. **A stray `#[cfg(test)]` anywhere
after line 778 truncated the control's view with both positive controls green.**
This round's own incident put such a line at **~607 — below 778, which is the only
reason a floor caught it.**

Three repairs, every figure re-measured after this round's additions (production
half now **4,613 lines / 228,785 bytes**):

1. a floor **PROPORTIONAL** to the file — 180,000 bytes, **78.7%, margin BELOW the
   measurement**, reached at line ~3,622;
2. a **DEEP anchor**, `fn forbidden_repo_path` at line ~4,592 of 4,613 — the one
   `fn scan_leading` at 374 could not be;
3. an **EXACTLY-ONE `#[cfg(test)]` sentinel-count assertion**, so the stripper's
   premise is a fact under test.

**Neither control was weakened.** `tests/spawn_seam_guard.rs` is UNEDITED and
`policy.rs`'s `SPAWN_ALLOWLIST` entry is intact.

## The mechanism pins — rounds 4, 5, 6 and 7 did NOT become dead code

- round 5's literalness bit — `Token.literal` FALSE for `pus?`, TRUE for `-c`,
  `include.path=/tmp/evil.cfg` and `status`. **The bit is RIGHT about every word
  of the bypass line**, and the refusal came from the new rule rather than from
  making the bit wrong;
- round 6's deletion model — `git >/dev/null push --force origin main` refused,
  `git x2>/tmp/o push --force origin main` PERMITTED;
- **`SEPARATORS` byte-identical** — zero hunks in this plan's entire `src/` diff —
  and `policy::is_separator(">")` still `false`;
- round 7's fail-closed grammar — `git --attr-source HEAD push --force origin main`
  at `force_push_blocked`, `git --bogus-opt status` at `envelope_assertion_failed`,
  `git --no-pager status` and `git - push --force origin main` at exit 0;
- Rule B's severed-head geometry, unchanged.

## The byte floors, unchanged

`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are at their values in `tests/envelope_wrapper_class.rs`, which this plan did not
touch. **`policy.rs`'s comment ratio was 22.37% before this plan wrote a line**,
so the 25% ratio assertion `19-20` deleted would have been RED at base. The
absolute floors are what the anti-vacuity control rests on, they are independent
of comment volume by construction, and this plan wrote more prose to that file
than `19-21` did. Neither was lowered and none was economised around.

## The decision region did NOT move, checked mechanically

- `git diff --stat` over `src/` is **exactly TWO files** — `policy.rs` and the one
  `cred.rs` doc block;
- `src/envelope/hooks.rs`, `tests/spawn_seam_guard.rs`, `Cargo.toml` and
  `Cargo.lock` have **no diff at all** (`T-19-SC` holds phase-wide);
- **no new `ParkReason` variant**;
- `first_unreadable_decision_word`, `config_key_operand_index`,
  `subcommand_word_indices`, `scan_gh_api` and `resolve_program_with_head` have no
  behavioural hunk and the post-filter gained no arm;
- **no second reading site was needed** — the refusal is produced inside
  `scan_leading` and returned through the existing channel;
- the guard's path shells out to nothing: every new `Command` use is inside
  `#[cfg(test)]`.

## Deviations

1. **The authorized corpus correction** (see the blocker section above). It is the
   one departure from this plan's additions-only prohibition, and it is also the
   one departure from the verification item *"`git diff --numstat` over `tests/`
   shows ZERO deletions"*: three of the four commits have zero deletions, and the
   correction necessarily replaced lines (53 insertions, 6 deletions). Recorded as
   a deviation rather than reported as clean.
2. **`ENVELOPE_ENV_DEFEATING_KEYS` lives inside `#[cfg(test)] mod tests`** rather
   than beside `ENVELOPE_ENV_KEYS`. `cargo clippy -- -D warnings` correctly
   flagged it as dead code in the production half, and a `#[cfg(test)]` attribute
   in place would have put a SECOND sentinel line in the production half —
   truncating the anti-vacuity stripper, which the same commit's sentinel-count
   assertion now forbids. It follows `GIT_GLOBAL_UNPROBED_OPTS`'s existing
   precedent and the reason is recorded in its doc.
3. **The `CONFIG_VALUE_OPTS` probe was redesigned mid-task.** The first shape
   asserted git must not echo the probe value back; that reads backwards for
   `--blob` (whose error names the value *because* it consumed it) and for
   `--default` (which returns the value as the default). Replaced with git's own
   classification, which is two-sided, needs no per-entry variants, and is
   strictly stronger. The abandoned shape is not in any commit.

## Found and NOT fixed

- **The three documented flakes** — two `driver_reattach`, one `envelope_tracer`
  ETXTBSY. **None fired**; not fixed; out of scope.
- **`cargo clippy --tests -- -D warnings`** still fails at base on four
  pre-existing lints in `src/browser.rs` and `src/project_creator.rs`. Not fixed,
  not the gate.
- **`FORGE_VALUE_OPTS` and `GH_API_VALUE_OPTS`** — left UNPINNED with the reason:
  `glab` is confirmed NOT INSTALLED, so a two-sided pin cannot run and a pin that
  skips is fail-open.
- **The `glab --host` forge cell** — carried forward **unfixed**, with its
  unconfirmed-callee caveat intact. Audit 7 explicitly declined to upgrade it
  without evidence and so does this plan. `subcommand_word_indices` untouched.

## `T-19-17r` — OUTSTANDING, and this plan did NOT accept it

Recorded as **still OUTSTANDING for the fourth round running**. **No
Accepted-Risks-Log row was added, `AR-19-13` was not created, and the word
"accepted" is not applied to `T-19-17r` anywhere in this plan's output.** Audits
5, 6 and 7 all confirmed the measurement and both pins and all three deliberately
declined to make the acceptance. Accepting a risk is a human decision.

## What remains uncovered

1. **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Four
   registered rows plus the persisted-alias arm, all re-measured at exit 0 this
   round (`git config alias.p "!git push --force origin HEAD:refs/heads/main"` and
   `git p`, walks EMPTY; `env $X push --force origin main` and
   `X=git; env $X push --force origin main`). Its pin is green and UNMODIFIED.
   **Closing `T-19-103` RESTORES layer 3's catch of the persisted-alias arm. A
   restoration is not a closure.**
2. **`T-19-91`** — OPEN at `high`, arms unweakened: `git reflog $S`,
   `git reflog show $S` and `git symbolic-ref $S` at exit 0 with **no second
   carrier**, and bare `git push $REF` at exit 0 in the in-namespace
   configuration. No decision-operand rule added, no denylist extended.
3. **`T-19-96`** — a glob in a push flag, one slot outside the decision region.
   Left exactly as pinned by `19-16`.
4. **`T-19-74`** — core rows frozen.
5. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision. The `cred.rs` doc correction is the ONE named
   exception.
6. **The `T-19-103` residue** — a future git adding a THIRD indirection section.
   Fails OPEN, **no automated control**, human re-audit only.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed. `T-19-86` and
`T-19-91` remain OPEN at `high`, so `/gsd-secure-phase 19` is NOT cleared.**

## Self-Check: PASSED

- `src/envelope/policy.rs` — FOUND (+1,196 lines across three commits)
- `src/envelope/cred.rs` — FOUND (one hunk at `:244-248`, doc lines only)
- `tests/envelope_config_resolution.rs` — FOUND (30 `#[test]` fns, +2 new)
- `19-SECURITY.md` — FOUND (+547 lines appended, 0 deletions, no audit table touched)
- `deferred-items.md` — FOUND (+117 lines appended, 0 deletions)
- commit `a8b8632` — FOUND
- commit `2313ecf` — FOUND
- commit `2448064` — FOUND
- commit `fe0d6c7` — FOUND
- commit `2190cf5` — FOUND
- `AR-19-13` row count in `19-SECURITY.md` — **0**
- `#[cfg(test)]` sentinel count in `policy.rs` — **1**
- `envelope_*` binaries run — **14**
- `passed + failed` — **1680** = 1673 + 7
