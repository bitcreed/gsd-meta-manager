---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 22
subsystem: envelope
tags: [security, corpus, config-resolution, RED, T-19-103, T-19-104, T-19-105]
status: complete

requires:
  - "19-21 (round 7's rule, and the 1639 baseline it recorded)"
  - "19-SECURITY.md audit 7 (T-19-103, T-19-104, T-19-105, T-19-106, T-19-107)"
provides:
  - "tests/envelope_config_resolution.rs — the SEVENTH evidence file, RED"
  - "CONFIG_RESOLUTION_CLASSES — a FOURTH named class axis with three alphabets and three mechanical fences"
  - "the exact handoff numbers 19-23 gates against: passed+failed = 1673, 34 new #[test] fns, 9 RED names"
affects:
  - "19-23 (the rules), which is gated on this plan's recorded RED state"

tech-stack:
  added: []
  patterns:
    - "corpus first, RED before the rule — the FIFTH two-plan split in this phase"
    - "post-fix verdicts asserted with a written derivation; underivable rows RECORDED, never asserted"
    - "a new axis gets a NEW named constant, NEW predicates and NEW floors beside byte-identical existing axes"

key-files:
  created:
    - tests/envelope_config_resolution.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md

decisions:
  - "The reproducers are built on LAYER-2-PERMITTED bases only; the five --force compositions are CONTROLS, asserted as such and fenced mechanically"
  - "A dotless -c key stays CONFINED — refusing it would turn round 7's whole generative property permanently red behind CALLEE_KNOWN_LEADING_PREFIX"
  - "The config axis is a FOURTH axis, not more entries in CALLEE_GRAMMAR_CLASSES: it is about what the verb runs under, not about the command line"
  - "GIT_CONFIG_NOSYSTEM recorded as a measured DEFEAT with an INERT harm — folded into T-19-104's class, never a new threat ID and never called a bypass"
  - "T-19-17r left OUTSTANDING with no AR-19-13 and no acceptance made"

metrics:
  duration: "~1h"
  completed: 2026-09-04

actuals:
  tokens: 121000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 22: The Corpus for git's Config Resolution — Summary

The seventh evidence file and a fourth named class axis, written and observed RED
before a single line of either rule exists.

## FIRST: `/gsd-secure-phase 19` is NOT cleared, and this plan closes NOTHING

**`T-19-86` and `T-19-91` both remain OPEN at `high`**, so this plan does not
clear `/gsd-secure-phase 19` — and neither will `19-23`, nor the two together.
`T-19-103`, `T-19-104`, `T-19-105`, `T-19-106` and `T-19-107` all stay OPEN at
this plan's end. `T-19-105` is closed only when `19-23`'s rules are certified by
the corpus written here, because a corpus is evidence about a control and there
are no controls yet.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" appears anywhere in this plan's output.

Closing `T-19-103` will be a **RESTORATION of layer 3's catch** — never a closure
of `T-19-86`.

## The commits

| SHA | What | `git show --stat` `src/` hunks |
|---|---|---|
| `de573cb` | `test(19-22)`: the seventh evidence file — the config the verb runs under, RED before the rules | **0** |
| `e7f3358` | `test(19-22)`: the fourth axis — `CONFIG_RESOLUTION_CLASSES`, three alphabets, three fences | **0** |
| `483f6e0` | `docs(19-22)`: the round-8 record — every row measured, nothing closed, no gate cleared | **0** |

`git diff --numstat HEAD~3..HEAD`: 2,083 + 1,215 + 336 + 184 insertions, **ZERO
deletions across all four files**. No file outside `files_modified` was touched;
`Cargo.toml` and `Cargo.lock` are untouched (`T-19-SC`).

## The gate arithmetic, stated and CHECKED

| | `passed + failed` | passed | failed | ignored | `envelope_*` binaries | result lines |
|---|---|---|---|---|---|---|
| before (`d0eb738`) | **1639** | 1639 | 0 | 13 | 13 | 42 |
| after | **1673** | 1664 | 9 | 13 | **14** | 43 |

`1673 = 1639 + 34`.

**New `#[test]` fns counted from `git show`: 28 in
`tests/envelope_config_resolution.rs` + 6 in `tests/envelope_wrapper_class.rs` =
34.** A red test RAN, so red→green leaves the total unchanged and every increase
comes ONLY from new fns — **the identity holds with no residual.**

Read with `rtk proxy cargo test --no-fail-fast` and `rtk proxy grep` over a
redirected log (D-34). `cargo build` and `cargo clippy -- -D warnings` both exit
0. `cargo clippy --tests` is NOT the gate: it fails at base on four pre-existing
lints in `src/browser.rs` and `src/project_creator.rs`, which were not touched.

## The COMPLETE RED list — `19-23`'s handoff contract

Nine names, and every one of them is one this plan created. **No other test in the
suite failed, and none of the three documented flakes fired** (their absence is
not evidence they are fixed).

`tests/envelope_config_resolution.rs` (7):

1. `after_19_23_a_config_indirection_carrier_is_refused_on_a_permitted_base`
2. `after_19_23_a_config_indirection_carrier_is_refused_on_the_in_namespace_push`
3. `after_19_23_the_environment_carrier_of_configuration_is_refused`
4. `after_19_23_the_disclosed_bare_word_cost_of_the_new_list_entry_is_refused`
5. `after_19_23_the_disclosed_cost_of_reading_the_section_and_not_the_variable_is_refused`
6. `after_19_23_the_scan_order_decides_which_clause_names_a_line_carrying_both`
7. `after_19_23_the_composition_rows_are_refused_which_proves_rounds_6_and_7_load_bearing`

`tests/envelope_wrapper_class.rs` (2):

8. `a_config_indirection_the_guard_cannot_bound_fails_closed_on_every_base`
9. `an_environment_carrier_of_configuration_fails_closed_on_every_base`

Verbatim from the failing output of row 3, as the shape of every one:

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

## Per-binary counts — all FOURTEEN `envelope_*` binaries RAN

| Binary | passed | failed | ignored |
|---|---|---|---|
| `envelope_advisory` | 10 | 0 | 0 |
| `envelope_argv_deletion` | 20 | 0 | 0 |
| `envelope_callee_grammar` | 19 | 0 | 0 |
| `envelope_command_position` | 18 | 0 | 0 |
| **`envelope_config_resolution`** | **21** | **7** | 0 |
| `envelope_credential` | 6 | 0 | 0 |
| `envelope_expansion_slots` | 32 | 0 | 0 |
| `envelope_hook_refusals` | 7 | 0 | 0 |
| `envelope_literal_decision` | 43 | 0 | 0 |
| `envelope_pr_cap` | 11 | 0 | 0 |
| `envelope_tracer` | 6 | 0 | 0 |
| `envelope_wiring` | 14 | 0 | 0 |
| `envelope_wrapper_bypass` | 13 | 0 | 0 |
| **`envelope_wrapper_class`** | **38** | **2** | 0 |

Fourteen, not thirteen — this plan's own evidence file executed.
`envelope_expansion_slots`, `envelope_command_position`,
`envelope_literal_decision`, `envelope_argv_deletion` and
`envelope_callee_grammar` are **fully GREEN and unmodified**, so rounds 4, 5, 6
and 7's evidence was not disturbed.

## THE FINDING THAT SHAPES THE WHOLE CORPUS, stated prominently

**A `T-19-103` reproducer must be built on a base LAYER 2 PERMITS.** Measured at
`d0eb738`, one fresh `GSD_MM_ENVELOPE_ROOT` per row, walk EMPTY on every one:

| Command | exit | reason id | walk |
|---|---|---|---|
| `git -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |
| `bash -lc "git -c include.path=/tmp/evil.cfg push --force origin main"` | 2 | `force_push_blocked` | EMPTY |
| `echo hi && git -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |
| `git >/dev/null -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |
| `git --attr-source HEAD -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |

**All five are CONTROLS.** They are refused for their VERB with the carrier never
examined. A corpus built out of them would be green before the fix and would
certify nothing — the eighth consecutive instance of `T-19-76`'s failure mode,
produced by the corpus rather than found by the next audit. The reproducers use
the three layer-2-PERMITTED bases audit 7 itself used, and
`tests/envelope_wrapper_class.rs` asserts that constraint **mechanically**.

## Every measured row

### `T-19-103` — the ten rows

| Command | exit BEFORE | reason id | pinned AFTER | walk |
|---|---|---|---|---|
| `git -c include.path=/tmp/evil.cfg commit -m x` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c includeIf.gitdir:/tmp/.path=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git --config-env=include.path=EVILVAR status` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git --config-env include.path=EVILVAR status` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c INCLUDE.PATH=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c "include.path=/tmp/evil.cfg" status` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.pathx=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=… -c core.hooksPath=/dev/null push --force origin main` | 2 | `hook_bypass_blocked` | 2 `envelope_assertion_failed` | EMPTY |

### `T-19-104` — the six rows

| Command | exit BEFORE | pinned AFTER | walk |
|---|---|---|---|
| `GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git commit -m x` | 0 | 2 `hook_bypass_blocked` | EMPTY |
| `GIT_CONFIG_PARAMETERS=… git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` | EMPTY |
| `export GIT_CONFIG_PARAMETERS=…; git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` | EMPTY |
| `env GIT_CONFIG_PARAMETERS=… git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` | EMPTY |
| `GIT_CONFIG_PARAMETERS="'include.path=/tmp/evil.cfg'" git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` | EMPTY |
| `echo GIT_CONFIG_PARAMETERS` | 0 | 2 `hook_bypass_blocked` | EMPTY |

### The eleven paired discriminators — asserted UNCHANGED

All at exit 2 `hook_bypass_blocked`, walks EMPTY: `git -c core.hooksPath=/dev/null
commit -m x`; `git --config-env=core.hooksPath=EVILVAR status`; `git --config-env
core.hooksPath=EVILVAR status`; `git -c CORE.HOOKSPATH=/dev/null status`;
`GIT_CONFIG_COUNT=0` in all three spellings before the in-namespace push; the same
three before `git status`; and `echo GIT_CONFIG_COUNT`.

**These are what make every `T-19-104` post-fix verdict DERIVABLE rather than
guessed.**

### Discrimination controls, disclosed cost, permitted half, dotless key, already-refused controls, compositions

| Command | exit BEFORE | reason id | pinned AFTER | what it is |
|---|---|---|---|---|
| `git -c includepath=/tmp/evil.cfg status` | 0 | (permit) | **0** | this round's `--signed no` |
| `git -c notinclude.path=/tmp/evil.cfg status` | 0 | (permit) | **0** | the same control, other side |
| `git -c include.pathx=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` | the DISCLOSED COST |
| `git -c user.name="$NAME" commit -m x` | 0 | (permit) | **0** | this axis's `ls {git,svn}-repo` |
| `git -c core.pager=cat log` | 0 | (permit) | **0** | permitted half |
| `git -c a=b status` | 0 | (permit) | **0** | DOTLESS KEY / `CALLEE_KNOWN_LEADING_PREFIX` |
| `git --git-dir=/tmp/g status` | 0 | (permit) | **0** | permitted half |
| `git -C /tmp status` | 0 | (permit) | **0** | permitted half |
| `git --no-pager status` | 0 | (permit) | **0** | round 7's control, carried here |
| `git -c a=b push --force origin main` | 2 | `force_push_blocked` | unchanged | verdict-preserving |
| `git -c a=b --attr-source HEAD push --force origin main` | 2 | `force_push_blocked` | unchanged | verdict-preserving |
| `git -c includeIf.gitdir:~/p/.path=… status` | 2 | `envelope_assertion_failed` | unchanged | **CONTROL** — existing rewriting-char clause |
| `git -c core.hooksPath=/dev/null -c include.path=… push --force origin main` | 2 | `hook_bypass_blocked` | unchanged | ordering pin, hooks key first |
| `git >/dev/null -c include.path=… status` | 0 | (permit) | 2 `envelope_assertion_failed` | composition, round 6 |
| `git --attr-source HEAD -c include.path=… status` | 0 | (permit) | 2 `envelope_assertion_failed` | composition, round 7 |
| `bash -lc "git -c include.path=… status"` | 0 | (permit) | 2 `envelope_assertion_failed` | composition, wrapper axis |
| `echo hi && git -c include.path=… status` | 0 | (permit) | 2 `envelope_assertion_failed` | composition, segment axis |
| `gh pr create --title x` | 0 | (permit) | unchanged | walk control: **one** ledger line, `alpha/pr-ledger.ndjson` (103 bytes); cap fires with **two** on call 2 |

### The `T-19-86` persisted-alias arm — RECORDED, NOT closed

| Command | exit | walk |
|---|---|---|
| `git config alias.p "!git push --force origin HEAD:refs/heads/main"` | 0 | EMPTY |
| `git p` | 0 | EMPTY |

Two separately-permitted tool calls the stateless guard cannot correlate. **Layer
3 catches the inner push TODAY** (`AR-19-03` working), **and that is precisely
what `T-19-103` removes.** `T-19-86` is NOT closed, narrowed or re-scoped.

## The real-git precedence table — the envelope's OWN injection as the control

`git version 2.43.0`. The control is the exact triplet `cred::hooks_path_env`
emits — count DERIVED from one pair — not a stand-in.

```
control:  GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS
          git config --get core.hooksPath                     -> /ENV_WINS
+ -c include.path=<file>                                      -> /INCLUDE_WINS
+ -c INCLUDE.PATH=<file>                                      -> /INCLUDE_WINS   <- CASE
+ -c includeIf.gitdir:<p>.path=<file>                         -> /INCLUDE_WINS
+ -c INCLUDEIF.gitdir:<p>.PATH=<file>                         -> /INCLUDE_WINS   <- CASE
+ --config-env=include.path=EVILVAR                           -> /INCLUDE_WINS
+ --config-env include.path=EVILVAR                           -> /INCLUDE_WINS
+ GIT_CONFIG_PARAMETERS="'core.hooksPath=/PARAM_WINS'"        -> /PARAM_WINS
+ GIT_CONFIG_PARAMETERS="'include.path=<file>'"               -> /PARAM_INCLUDE_WINS
+ -c include.pathx=<file>                                     -> /ENV_WINS  (git IGNORES it)
+ -c notinclude.path=<file>                                   -> /ENV_WINS
persisted `git config include.path <file>`, alone            -> /INCLUDE_WINS
persisted `git config include.path <file>`, under injection  -> /ENV_WINS  (INERT)
git -c a=b version   -> git version 2.43.0, rc 0
git -c a=b config --get a -> error: key does not contain a section: a, rc 1
```

**The two CASE rows are the point**, and the persisted-`git config` row is why the
`git config` subcommand is **not** a second escape and correctly needs no clause.
All of this is asserted as tests, not merely recorded, so a later git that changed
the behaviour turns the file red rather than leaving a stale sentence.

## The case-insensitivity question, ANSWERED — and no existing pin changes

**Yes: key comparison must be ASCII-case-insensitive on the SECTION and the
VARIABLE, and the SUBSECTION must not be read at all.** Git folds the section and
the variable and leaves the subsection case-sensitive, measured in both halves.

**No existing pin changes.** `is_hooks_path_key` (`policy.rs:750-752`) is already
`eq_ignore_ascii_case("core.hookspath")` and `git -c CORE.HOOKSPATH=/dev/null
status` is measured at `hook_bypass_blocked` today. The guard's whole-key fold is
MORE permissive than git for a subsectioned key — the safe direction.

## The class confirmed END TO END, reproduced not cited

A claim in this codebase about which forms outrank the injection is already known
FALSE (`cred.rs:241-248`), so this round reproduced the behaviour rather than
repeating audit 7's sentence. Fixture rebuilt, hook delivered exactly as the
envelope delivers it, remote SHA recorded before and after each leg:

```
leg 1  plain in-namespace push        rc=1, hook REFUSED   11b417c -> 11b417c  UNMOVED
leg 2  -c include.path=<evil>         rc=0                 11b417c -> 0e482a9  MOVED
leg 3  GIT_CONFIG_PARAMETERS carrier  rc=0                 0e482a9 -> ba3c923  MOVED
leg 4  pre-commit: control rc=1 (refused), carrier rc=0     ba3c923 -> 79a7c23  MOVED
```

**All four legs reproduced. None failed.**

## The two rows RECORDED rather than asserted, and why each is left for `19-23`

1. **`GIT_CONFIG_NOSYSTEM=1 git push origin refs/heads/gsd-auto/alpha/w`** — exit
   0 today, walk EMPTY. Whether it earns an `ENVELOPE_ENV_KEYS` entry is
   `19-23`'s **design decision**, because the defeat is real and the harm is
   inert. Asserting merely that it is refused would be asserting a verdict this
   plan cannot derive.
2. **`git -c include.path=$F push --force origin main`** — exit 2
   **`force_push_blocked`** today, refused for its VERB rather than its carrier.
   Its verdict does not move while its IDENTIFIER may; which of the two it lands
   on is a clause-ordering outcome `19-23` measures.

Both are driven through `record_only` and **printed, never asserted**. `19-23`
appends their pins as new fns.

## The design question, with the shapes this corpus turns red

**The guard must not ask "does this assignment spell `core.hooksPath`" but "can I
establish what `core.hooksPath` will be."** The costing is `19-23`'s. What this
corpus now turns RED:

- a **substring match on `include`** — red at `includepath` and `notinclude.path`;
- a **blanket refusal of every `-c`** — red at the permitted half;
- a **refusal of a key that cannot be decomposed into a section** — red at
  `-c a=b` AND at round 7's entire generative property;
- a refusal raised in a **SECOND PASS** over the leading tokens — red at one of
  the two ordering pins, which are pinned at deliberately different identifiers.

And the cost pinned in the other direction: `git -c include.pathx=…`, which real
git IGNORES, is pinned REFUSED.

**The residue has NO automated control.** A future git adding a THIRD indirection
section is not covered and the rule fails OPEN on it. The precedence pin cannot
observe a section it does not name — it holds only the two known sections'
behaviour, in the reverse direction. **Re-audit is the compensating control, and
there is no other.** (This is the plan-check WARNING, applied.)

## `GIT_CONFIG_NOSYSTEM` — a measured DEFEAT with an INERT harm

```
GIT_CONFIG_SYSTEM=<file with credential.helper=evil> git config --get credential.helper -> evil
  + GIT_CONFIG_NOSYSTEM=1                                                               -> rc 1, nothing read
```

Guard-side permitted at exit 0. **The harm is INERT and is not claimed**:
`cred::write_gitconfig` (`cred.rs:253-271`) points BOTH `GIT_CONFIG_GLOBAL` and
`GIT_CONFIG_SYSTEM` at the same helper-free file, so suppressing the system read
removes a deny the global pointer duplicates. Folded into `T-19-104`'s class with
the `19-14` provenance caveat — **never a new threat ID and never called a
bypass.**

## Why the existing `ENVELOPE_ENV_KEYS` drift pin cannot see `T-19-104`

It is sourced from `cred::EnvelopeEnv::with_run_id(build_env_in(…))` — the keys
the envelope **SETS or REMOVES** — and every floor it carries is over the
envelope's own entries. Both cells here are keys the envelope **neither sets nor
removes but which DEFEAT ones it does**. **The fix is a SECOND SOURCE, not a wider
filter**, and it is `19-23`'s.

## The fourth axis, as built

`CONFIG_RESOLUTION_CLASSES` — five classes with degenerate-proof predicates,
standing beside a **byte-identical** `UNREADABLE_CLASSES`, `DELETION_CLASSES` and
`CALLEE_GRAMMAR_CLASSES`. Three alphabets, asserted mutually disjoint:

- `CONFIG_CONFINED_CARRIERS` (5) — the **only** one the invariance arm draws;
- `CONFIG_INDIRECTION_CARRIERS` (5) — own fail-closed property;
- `CONFIG_ENV_CARRIERS` (3) — spliced as an **assignment prefix**, own property.

Three mechanical fences, all GREEN today:

- the **DOTLESS FENCE** — no indirection entry may carry a dotless key half, and
  `-c a=b` is asserted PRESENT in the confined alphabet, with the message naming
  `CALLEE_KNOWN_LEADING_PREFIX`, its line and the consequence;
- the **PERMITTED-BASE FENCE** — `status`, `commit -m x` and the in-namespace push
  asserted present by name and driven to exit 0, with the five
  `force_push_blocked` compositions recorded as controls in the message;
- the **DISJOINTNESS** assertions across all three alphabets.

Floors with the arithmetic stated: **161 cases** (69 refused-base, 92
permitted-base) over **21 slots**; per-class **70 / 105 / 21 / 28 / 28**. Measured
counts printed by the corpus: the invariance arm drove **70 verdict-preserving
cases** (40 permits, 30 `force_push_blocked`, walks EMPTY throughout); the
indirection arm's **40 permitted-base cases** and the environment arm's **12** are
what make the two properties RED.

## Mechanism pins — all five present and non-vacuous

- round 5's literalness bit — `Token.literal` FALSE for `pus?`, TRUE for `-c`,
  `include.path=/tmp/evil.cfg` and `status`;
- round 6's deletion model — `git >/dev/null push --force origin main` refused,
  `git x2>/tmp/o push --force origin main` PERMITTED;
- `policy::is_separator(">")` is `false`, `SEPARATORS` byte-identical (zero `src/`
  hunks), `<` and `>>` also false, `&`/`&&` true for non-vacuity;
- round 7's fail-closed grammar — `git --attr-source HEAD push --force origin
  main` at `force_push_blocked`, `git --bogus-opt status` at
  `envelope_assertion_failed`, `git --no-pager status` and `git - push --force
  origin main` both at exit 0;
- Rule B — `head_is_command_position == false` for the three severed spellings and
  `true` for `{ git status; }`.

## The byte floors, re-measured

| File | raw | stripped | floor | ratio |
|---|---|---|---|---|
| `src/envelope/policy.rs` | 307,496 | 68,785 | 40,000 | **22.37%** |
| `src/envelope/hooks.rs` | 99,909 | 33,460 | 20,000 | 33.49% |

**The 25% ratio assertion `19-20` deleted would be RED TODAY.** Both floors
unchanged; this plan was granted no deletion and took none.

## Rows expected RED that came up GREEN

**None.** Every audit-7 row reproduced at its recorded verdict, every cell found
while planning reproduced, and all four end-to-end legs reproduced. Nothing had to
be reported as a finding against the plan's numbers.

## Found and NOT fixed

- **`T-19-106`** — `git config --comment core.hooksPath /dev/null` at exit 0 while
  real git answers ``error: unknown option `comment'``. Registered, not fixed.
- **`T-19-107`** — `git -v` at exit 2 `envelope_assertion_failed` while real git
  runs it at rc 0. Registered, not fixed.
- **`cred.rs:241-248`** still claims *"the one form that outranks this injection
  is `git -c core.hooksPath=… push`"*, which four measured forms falsify. **Not
  corrected here** — that is `19-23`'s named, narrow exception to this phase's
  `cred.rs` fence, and it belongs in the plan that can measure the rule it
  describes.
- **`glab --host`** — `glab` confirmed NOT installed, callee unconfirmed,
  deliberately not upgraded. `FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and
  `subcommand_word_indices` untouched.
- **The three documented flakes** — two `driver_reattach`, one `envelope_tracer`
  ETXTBSY. None fired; not fixed; out of scope.
- **`cargo clippy --tests -- -D warnings`** still fails at base on four
  pre-existing lints in `src/browser.rs` and `src/project_creator.rs`. Not fixed.

## `T-19-17r` — OUTSTANDING, and this plan did NOT accept it

`19-17-SUMMARY.md` calls it "accepted". Audits 5, 6 and 7 all confirmed the
measurement and both pins at `tests/envelope_literal_decision.rs:1355-1379` and
all three deliberately declined to make the acceptance. **There is no
`AR-19-13`, no Accepted-Risks-Log row and no register row, and this plan added
none.** The word "accepted" is not applied to `T-19-17r` anywhere in this plan's
output. Accepting a risk is a human decision.

## What remains uncovered

1. **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Four
   registered rows plus the **persisted-alias arm recorded here and NOT closed**,
   all at exit 0. Closing `T-19-103` restores layer 3's catch; it does not close
   `T-19-86`.
2. **`T-19-91`** — OPEN at `high`, arms unweakened: `git reflog $S`, `git reflog
   show $S` and `git symbolic-ref $S` at exit 0 with **no second carrier**, and
   bare `git push $REF` at exit 0 in-namespace. No decision-operand rule added, no
   denylist extended.
3. **`T-19-96`** — a glob in a push flag, one slot outside the decision region.
   Registered by `19-16`, not fixed.
4. **`T-19-74`** — core rows frozen and re-measured permitted.
5. **`T-19-84`, `T-19-85`, `T-19-61`…`T-19-73`** — open and unaccepted by explicit
   user decision.
6. **`T-19-103`, `T-19-104`, `T-19-105`, `T-19-106`, `T-19-107`** — open; the
   corpus exists and is RED, and `19-23` writes the rules.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` remain OPEN at `high`, so `/gsd-secure-phase 19` is not cleared.

## Self-Check: PASSED

- `tests/envelope_config_resolution.rs` — FOUND (2,083 lines, 28 `#[test]` fns)
- `tests/envelope_wrapper_class.rs` — FOUND (+1,215 lines, 6 new `#[test]` fns, 0
  deletions)
- `19-SECURITY.md` — FOUND (+336 lines appended, 0 deletions, no audit table
  touched)
- `deferred-items.md` — FOUND (+184 lines appended, 0 deletions)
- commit `de573cb` — FOUND
- commit `e7f3358` — FOUND
- commit `483f6e0` — FOUND
- `git show --stat` `src/` hunk count across all three commits — **0**
- `git diff --numstat HEAD~3..HEAD` deletions — **0**
