---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 25
subsystem: envelope
tags: [security, config-resolution, reparsed-value, T-19-108, T-19-109, T-19-106, T-19-105, T-19-110]
status: complete
requires:
  - "19-24's corpus, observed RED and committed with zero src/ hunks"
  - "19-23's confinement clause (round 8), non-dead and pinned"
provides:
  - "REPARSED_COMMAND_SECTIONS + config_key_names_a_reparsed_command_section"
  - "the RE-PARSE clause at BOTH existing decision regions"
  - "a revisit condition + version witness on both config-section constants"
  - "cred.rs's counted claim replaced by a statement of REGIONS"
  - "the GH_API_VALUE_OPTS two-sided pin with both negative controls"
  - "T-19-110, a NEW open finding this plan made and did not fix"
affects:
  - "audit 9 / a future round, which inherits T-19-110 and the three fail-open directions"
tech-stack:
  added: []
  patterns:
    - "a rule's REACH is derived from a measurement of the callee, never from its docs"
    - "a residue is stated and handed to no control; a schedule is not a control"
    - "record, do not close: a finding outside the round's class is registered, not fixed"
key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/envelope/cred.rs
    - tests/envelope_reparsed_value.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "Option (b) adopted; option (a) rejected on three MEASURED grounds, not preference"
  - "The carrier must be read, not just the first byte — measured, --config-env=alias.q='!EVIL' resolves /INCLUDE_WINS"
  - "Region 2 gates on !is_read plus a VALUE WORD, because is_write is blind to a dash-leading value"
  - "T-19-110 recorded and NOT fixed: widening is_write moves verdicts with no corpus able to fail on them"
  - "The version witness is a SCHEDULE, never a CONTROL, at every site it is mentioned"
metrics:
  duration: ~3h
  completed: 2026-09-04
actuals:
  tokens: 30608
  tasks: 4
  commits: 4
---

# Phase 19 Plan 25: The Rules for a Value the Guard Confined — Summary

Git re-parses exactly one kind of config value as a command line, and the guard
now establishes that fact rather than inheriting it — at the two decision regions
that already read the same word.

## STATED FIRST, as the plan requires

**`/gsd-secure-phase 19` is NOT cleared by this plan.** `T-19-86` remains **OPEN
at `high`** — recorded by audit 8 as measurably WIDER than the register credits —
and `T-19-91` remains **OPEN at `high`** with its arms unweakened. `T-19-96` is
registered open, `T-19-74`'s core rows are frozen, and `T-19-61` … `T-19-73`,
`T-19-84` and `T-19-85` are open and unaccepted. **Only the WRAPPER-OPERAND
sub-class of `T-19-60` is closed**; no unqualified "T-19-60 is closed" appears
anywhere in this plan's output.

**`T-19-108` closes only AS SCOPED, and audit 7's `!`-bodied destructive pair
STILL WORKS after this plan:**

```
git config alias.q '!git -c include.path=<evil> push --force origin HEAD:refs/heads/main'
git q
```

A `!` body is a whole command line handed to a governed program as DATA, which is
**`T-19-86`** — out of scope by explicit user decision, with both its rows pinned
PERMITTED in files this plan may not edit. **Re-measured in this plan's own gate
run against the corpus's rebuilt bare remote, that pair still moved the ref:
`d7cd9d7` → `258650e`.**

## The carry-forward RED, confirmed still RED — the plan's mandated first action

Re-run against the unmodified tree at `57d715b`, **before any production line
moved**. Verbatim:

```
tests/envelope_reparsed_value.rs
failures:
    after_19_25_a_persisted_alias_carrying_an_indirection_is_refused_in_every_spelling
    after_19_25_a_reparsed_alias_value_carrying_an_indirection_is_refused
    after_19_25_a_reparsed_alias_value_reaching_the_by_name_hooks_deny_is_refused
    after_19_25_defining_an_ordinary_non_shell_alias_is_refused_and_its_twin_is_not
    after_19_25_the_case_varied_and_second_carrier_spellings_are_refused_too
    after_19_25_the_scan_order_decides_which_clause_names_a_line_carrying_both
    after_19_25_the_three_non_shell_boundary_spellings_are_refused

test result: FAILED. 22 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out

tests/envelope_wrapper_class.rs
failures:
    a_carrier_inside_a_confined_config_value_fails_closed_in_both_deliveries

test result: FAILED. 40 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

**Eight RED, matching `19-24-SUMMARY.md`'s recorded handoff set exactly. No row
expected RED came up green.**

### The green transition, with the clause that produced each

```
tests/envelope_reparsed_value.rs  test result: ok. 34 passed; 0 failed
tests/envelope_wrapper_class.rs   test result: ok. 41 passed; 0 failed
```

| RED name | before | after | reason id | clause |
|---|---|---|---|---|
| `…a_reparsed_alias_value_carrying_an_indirection_is_refused` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1 |
| `…a_reparsed_alias_value_reaching_the_by_name_hooks_deny_is_refused` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1 |
| `…the_case_varied_and_second_carrier_spellings_are_refused_too` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1: section fold + `--config-env` unreadable-value arm |
| `…the_three_non_shell_boundary_spellings_are_refused` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1: git's one-byte rule |
| `…the_scan_order_decides_which_clause_names_a_line_carrying_both` | exit 2 `hook_bypass_blocked` | exit 2 | `envelope_assertion_failed` / `hook_bypass_blocked` | region 1, ordered before `is_hooks_path_key` |
| `…a_persisted_alias_carrying_an_indirection_is_refused_in_every_spelling` | exit 0 | exit 2 | `envelope_assertion_failed` | **region 2** |
| `…defining_an_ordinary_non_shell_alias_is_refused_and_its_twin_is_not` | exit 0 | exit 2 | `envelope_assertion_failed` | regions 1 **and** 2 |
| `a_carrier_inside_a_confined_config_value_fails_closed_in_both_deliveries` | exit 0 | exit 2 | (exit code + empty walk) | both regions |

**Region 1 alone moved five of seven.** The two that remained were exactly the
persisted rows — the mechanical confirmation of `19-24`'s deviation-1 finding that
a rule inside `scan_leading` alone closes only ONE of the two deliveries.

## Commits — four, in order

| # | SHA | What |
|---|---|---|
| 1 | `bd8be7c` | the RE-PARSE clause at region 1 — `scan_leading` |
| 2 | `6174ce7` | region 2 at `classify_config`, and the two derived pins |
| 3 | `b45c2a2` | `T-19-109`'s region correction, `T-19-106`'s pin and kept entry |
| 4 | `089fb18` | the record, the registrations, and the gate |

`git diff --stat HEAD~4..HEAD` — **five files, exactly `files_modified`, nothing
outside it, and neither `Cargo.toml` nor `Cargo.lock`:**

```
19-SECURITY.md                    | 453 ++++++++
deferred-items.md                 | 142 +++
src/envelope/cred.rs              |  79 +-
src/envelope/policy.rs            | 1214 +++++++++++++++++++-
tests/envelope_reparsed_value.rs  | 294 +++++
5 files changed, 2148 insertions(+), 34 deletions(-)
```

**Zero deletions under `tests/`** (`294 / 0`), and
`tests/envelope_command_position.rs` and `tests/envelope_config_resolution.rs`
show **zero diff lines**. Both `T-19-86` `!`-bodied rows are green and unmodified.

## The gate — arithmetic STATED and CHECKED

`rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep` over a
redirected log (D-34).

| | |
|---|---|
| `19-24`'s recorded `passed + failed` | **1710** |
| observed `passed + failed` | **1720** (1720 passed, **0** failed, 13 ignored) |
| new `#[test]` fns from `git show` | 3 (`bd8be7c`) + 5 (`6174ce7`) + 2 (`b45c2a2`) = **10** |
| identity | 1710 + 10 = **1720** ✓ no disagreement |

**A red test RAN, so red→green leaves the total unchanged; every increase comes
only from new `#[test]` fns.** **ZERO failures — no documented flake fired**,
which is not evidence they are fixed; the two `driver_reattach` failures and the
`envelope_tracer` `ExecutableFileBusy` race remain pre-existing, environmental and
out of scope, and none was touched. `cargo build` and
`cargo clippy -- -D warnings` exit 0 (`cargo clippy --tests` is NOT the gate and
still fails at base on four pre-existing lints in `src/browser.rs` and
`src/project_creator.rs`).

### Per-binary counts — all FIFTEEN `envelope_*` binaries RAN

| binary | run | | binary | run |
|---|---|---|---|---|
| `envelope_advisory` | 10 | | `envelope_pr_cap` | 11 |
| `envelope_argv_deletion` | 20 | | **`envelope_reparsed_value`** | **34** |
| `envelope_callee_grammar` | 19 | | `envelope_tracer` | 6 |
| `envelope_command_position` | 18 | | `envelope_wiring` | 14 |
| `envelope_config_resolution` | 30 | | `envelope_wrapper_bypass` | 13 |
| `envelope_credential` | 6 | | **`envelope_wrapper_class`** | **41** |
| `envelope_expansion_slots` | 32 | | | |
| `envelope_hook_refusals` | 7 | | | |
| `envelope_literal_decision` | 43 | | | |

Every count matches `19-24`'s except `envelope_reparsed_value`, 29 → 34, the five
new fns this plan added.

## The design question, ANSWERED: option (b)

**Option (a) — scan the confined value recursively with a stated depth bound — is
REJECTED on three MEASURED grounds:**

1. **It needs a SECOND TOKENIZER.** Git splits an alias body with its OWN rules —
   `-c alias.m='config --get "core.hooksPath"' m` RESOLVES where the literally
   quoted key is an `invalid key` error.
2. **It needs UNBOUNDED RECURSION.** Depth 2 **and depth 3** both resolve
   `/INCLUDE_WINS`, so any depth bound leaves a residue **an attacker reaches
   today by adding one nesting level** — strictly worse than round 8's, which
   needs a future git.
3. **It cannot read a body it does not have.** `--config-env=alias.q=<VAR>` and
   `git -c alias.q` deliver no readable body, so (a) collapses to (b) at exactly
   the carriers that matter.

**Option (b) — ADOPTED**: an assignment whose key names a value git re-parses as a
command line is one the guard cannot bound.

## K1 / K2 / K3 — re-measured at this plan's base, not inherited by citation

Against `git version 2.43.0`, with `cred::hooks_path_env`'s own triplet as the
control:

```
control, no carrier                                              -> /ENV_WINS
-c alias.a='-c include.path=<f> config --get core.hooksPath' a   -> /INCLUDE_WINS   K1
-c alias.b='!git config --get core.hooksPath' b                  -> /ENV_WINS       K2
-c alias.g='config --get core.hooksPath !x' g                    -> /ENV_WINS
-c alias.q='"!git config --get core.hooksPath"' q  (QUOTED)      -> expansion failed
-c alias.t='<TAB>!git …' t                                       -> expansion failed
```

* **K1** — re-parsed as a GIT command line, IN-PROCESS, including leading options.
  `alias.<name>` with a non-`!` body. **The only member**, and the whole of
  `T-19-108`.
* **K2** — re-parsed as a SHELL command line run as a CHILD that **INHERITS** the
  injection, so it needs no entry. That is a **measurement**, not a category
  argument. `core.pager` and `core.editor` are **not claimed**.
* **K3** — a FILE PATH spliced at the directive's precedence. Closed by `19-23`.

So the constant names **one section**, because that is where K1 lives.

## The two-region rule, and why region 2 is REQUIRED

Region 1 is `scan_leading`'s existing assignment block, ordered after the
confinement clause and **before** `is_hooks_path_key`, on the channel that already
carried three refusals. Region 2 is `classify_config`'s key operand — the operand
`is_hooks_path_key` already reads and already refuses
`git config core.hooksPath /dev/null`.

**The persistence asymmetry is what makes region 2 load-bearing.** A persisted
include must WIN a precedence contest against the injection and LOSES (audit 8:
INERT at repo-local, `--worktree`, GLOBAL). **An alias only has to EXIST.** **No
`include.path` or `includeIf` clause was added at region 2**, and
`git config include.path /tmp/evil.cfg` is pinned PERMITTED twice so adding one
lands red.

### No third reading site — confirmed mechanically

No hunk in `first_unreadable_decision_word`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api` or `resolve_program_with_head`.
`src/envelope/hooks.rs` has **zero diff lines**. **No `ParkReason` variant added**
— the refusal is `EnvelopeAssertionFailed`, with a message naming THIS mechanism
rather than the include one (D-24). **The two ordering rows landed at their
deliberately DIFFERENT identifiers**, which is the mechanical proof the clause is
raised inside the ONE left-to-right walk.

## The three fail-open directions — stated, and handed to NO control

Written into the constant's doc, the predicate's doc, the value helper's doc and
the record, in these words:

1. **An `alias.*` already present in a config file the guard never saw a write
   to.** Stateless, argv-only; a repo-local `.git/config` alias predating the run
   is LIVE. `T-19-86`'s shape.
2. **A `!`-bodied body carrying its own carrier** — audit 7's pair, still working.
   `T-19-86`.
3. **A future git that re-parses a SECOND config value as a git command line.**

**None is handed to the pin or to any other control.** The real-git pin holds only
the REVERSE direction and its own doc opens by saying so, because it iterates the
constant's entries and an entry that does not exist is never probed.

### The revisit condition and the version witness

Both constants gained a revisit condition in `AR-19-03`'s shape, plus
`the_config_section_constants_record_the_git_version_they_were_derived_against`,
which **cannot skip, cannot warn without failing, and does not pass when `git` is
absent**. Its cost is stated: **it fires on every git upgrade, including harmless
ones — and that IS the schedule.**

**THE WITNESS IS A SCHEDULE, NOT A CONTROL.** It observes exactly one bit — that
the installed version string moved — so it can say **WHEN to look**. **It cannot
say WHAT changed.** It does not observe a third indirection section or a second
re-parsed config value appearing, and **it stays GREEN on a git that adds one
without changing its version string.** The no-control claim is therefore
**unchanged and stands beside it**: there is NO automated control over any of the
three directions, and the witness schedules the human re-audit that is the only
control there is.

**Provenance, recorded rather than presented as a sentence that was always right:**
an earlier draft of `19-25` stated the no-control claim and added the witness in
the same breath, in a way that read as the witness BEING the control —
**`T-19-107`'s own shape arriving in the round that inherited it** — and a
plan-check caught it, exactly as a plan-check caught round 7's residue hidden
behind a pin that could not observe it.

**No acceptance was made, no `AR-` row was added** (`AR-19-13` count: **0**).

## The cost, measured from BOTH sides

**Defining a non-shell alias is refused; using one is not. Ordinary aliases do NOT
all keep working, and saying so plainly is the point.**

| refused after `19-25` (a DEFINITION) | permitted twin (same work) |
|---|---|
| `git -c alias.st=status st` | `git status` |
| `git -c alias.lg="log --oneline" lg` | `git log --oneline` |
| `git -c alias.co=checkout co` | `git checkout` |
| `git config alias.co checkout` | `git checkout` |
| `git config alias.st status` | `git status` |
| `git config alias.lg 'log --oneline'` | `git log --oneline` |

**INVOCATION unchanged at exit 0**: `git p`, `git co`, `git st`, `git lg`,
`git z`, `git q`. Round 8's permitted half is green; round 7's generative property
is green behind `CALLEE_KNOWN_LEADING_PREFIX = "-c a=b"`; `git -c aliasx.q=…` and
`git -c notalias.q=…` stay at exit 0 (a prefix test reddens the first, a substring
test the second).

**Two further disclosed over-refusals**: `git -c alias.q status` (no `=`) is
refused although real git resolves the control value — **no harm behind that
refusal**, and the corpus says so; and every `--config-env` alias assignment is
refused because its value half is a variable NAME the guard does not read.

## A carrier-reading requirement that was MEASURED, not assumed

```
git --config-env=alias.q='!EVIL'   with a NON-`!` body in `!EVIL`   ->  /INCLUDE_WINS
```

**A first-byte test applied uniformly across both carriers would read the `!` of a
variable NAME as git's shell rule and FAIL OPEN on exactly that row.** So the value
helper reads the CARRIER — from `argv[index]`, the token the loop already holds —
and treats every `--config-env` value as unreadable. **No new reading site, no
second pass, no arm added to `leading_git_option`.** Pinned from both sides in
`policy.rs`'s own real-git pin.

## `T-19-109` — a count replaced by REGIONS, as a NAMED NARROW exception

`cred.rs`'s limit paragraph claimed **"FIVE forms outrank this injection"**.
Measured, there is a **SIXTH** — a non-`!` `alias.<name>` body beginning
`-c include.path=<file>` — so the counted claim was wrong the day it was written,
**for the second time** (it first claimed ONE). It now states the REGIONS each
closure covers and what is NOT covered, **counting nothing**, keeping the
client-side / branch-protection closing sentence.

**Recorded as an exception rather than presented as ordinary scope.** **ZERO
non-doc lines changed**, verified by `git diff --unified=0` filtered for lines that
are not `///` returning **0**. `T-19-61` … `T-19-73` are NOT taken on; no other
item in the file was opened.

**The "RESTORATION of layer 3's catch" correction is recorded BESIDE the 19-22
record, the 19-23 record and `19-23-SUMMARY.md` — none of them was edited**, and
`19-24-SUMMARY.md` was not edited either. The claim is true only for an alias body
carrying no carrier of its own; the pin measures both halves.

## `T-19-106` — both halves closed

**The `gh api` pin.** All seventeen entries answer ``flag needs an argument``
against real `gh` 2.45.0, probed **ENDPOINT-LESSLY** so **no row touches the
network** — the endpoint-bearing spelling makes a real HTTP request and would be
non-hermetic and fail-open without network or auth. **Both negative controls, with
their MEASURED strings pinned rather than paraphrased:**

```
gh api --bogus-opt   -> unknown flag: --bogus-opt
gh api --paginate    -> accepts 1 arg(s), received 0
```

The second is a **POSITIONAL** error, which is why it is the right control: `gh`
**accepted** the flag, consumed **no** value, and reached the missing-endpoint
complaint. **Provenance recorded**: both plans first described it as a flag-level
answer and a plan-check corrected them.

**`--hostname` is KEPT in `FORGE_VALUE_OPTS` — a correction to audit 8's own
suggestion, re-measured against the built binary:**

```
glab --hostname gitlab.com mr create --title x   -> exit 0, exactly ONE pr-ledger line
glab --host     gitlab.com mr create --title x   -> exit 0, ZERO pr-ledger lines
```

Removal moves a counted creation form to **UNCOUNTED** (`T-19-35`). The asymmetry
is pinned: `gh pr create --hostname` answers ``unknown flag: --hostname``, so the
entry is **INERT for `gh`** and **load-bearing for `glab`**, whose callee is
**UNCONFIRMED because `glab` is not installed** (`command -v glab` re-run, found
nothing). **The `gh` half is pinned two-sided; the `glab` half cannot be pinned on
this machine and is NOT.** The `glab --host` cell is carried forward UNFIXED.

## Bare-remote SHAs measured

This plan built **no new** bare-remote fixture; the corpus's own fixture runs in
every gate run and was reproduced at the post-fix tree:

```
LEG 1 CONTROL plain push            ok=false  004e6bb -> 004e6bb  (unmoved)
LEG 2 CONTROL alias, no carrier     ok=false  004e6bb -> 004e6bb  (unmoved)
LEG 3 CARRIER persisted alias       ok=true   004e6bb -> 62a23b0
LEG 4 CARRIER -c alias delivery     ok=true   62a23b0 -> d7cd9d7
LEG 5 pre-commit carrier                      d7cd9d7 -> 35d9b21
T-19-86 `!` pair (NOT CLOSED)       ok=true   d7cd9d7 -> 258650e
```

Legs 3–5 measure **git**, not the guard, and the last row is the one that matters
here: **audit 7's pair still moves the ref after this round.**

## The mechanism pins and the anti-vacuity floors, re-measured

Rounds 5, 6, 7 and 8's mechanisms are all non-dead and green. `SEPARATORS` is
**byte-identical** at its list and `policy::is_separator(">")` is `false`. Every
`Token`/`Segment` bit is unchanged. Round 8's confinement clause is non-dead with
both discrimination controls green.

| | measured | floor |
|---|---|---|
| `policy.rs` production half | **261,386** bytes | `POLICY_MIN_PRODUCTION_BYTES = 40_000`, proportional `>= 180_000` |
| `hooks.rs` production half | **74,357** bytes | `HOOKS_MIN_PRODUCTION_BYTES = 20_000` |
| deep anchor | `fn forbidden_repo_path` at production line **5,141** of **5,162** | present |
| sentinel | exactly **ONE** `#[cfg(test)]`, at line **5,163** | count asserted |

Nothing was lowered; this plan only GREW the production half.

## Known Stubs

None. Every assertion this plan writes was measured against the built binary or
against real `git`/`gh` before being written, and the two rows `19-24` could not
derive were measured post-fix and pinned at what was measured.

## Deviations from Plan

**1. [Rule 1 — measured correctness] The value test reads the CARRIER as well as
the first byte.** The plan's `read_first` note said the clause "needs no
carrier-specific arm". That is true of the KEY check — both carriers reach it —
but **NOT of the VALUE test**, and the plan's own action text mandates that a
`--config-env` value be treated as unreadable. Measurement forced the point:
`git --config-env=alias.q='!EVIL'`, with a non-`!` body in the variable `!EVIL`,
resolves `/INCLUDE_WINS`, so a uniform first-byte test fails OPEN there. The
carrier is read from `argv[index]` — the token the loop already holds — so there is
still **no new reading site, no second pass, and no arm added to
`leading_git_option`.** Pinned from both sides. Commit `bd8be7c`.

**2. [Rule 1 — plan instruction unsatisfiable as written] Region 2 gates on
`!is_read` plus a VALUE WORD, not on `is_write`.** The plan required the clause
"under the existing `is_write && !is_read` gate". **That gate is unsatisfiable for
`T-19-108`'s headline persisted row.** `scan_config`'s walk treats any word
starting with `-` as an option, so `is_write`'s classic-form test never sees a
dash-leading value, and `git config alias.p '-c include.path=<f> …'` leaves
`is_write` FALSE — while real git accepts the write and `git p` then resolves
`/INCLUDE_WINS`. Rather than edit an evidence row, the clause reads the **VALUE
WORD** via the new `ConfigScan::value_word`, indexed off `key_operand`'s own index
into the **same** walk. `config_key_operand_index` is unchanged and no index
primitive moved. Commit `6174ce7`.

**3. [Rule 1 — probe correctness, recorded rather than hidden] The K2 arm of the
real-git pin was corrected.** It was first written with the body
`!git -c include.path=<f> config --get core.hooksPath` and measured
`/INCLUDE_WINS`. That was **not** a contradiction of the `!` carve-out — it is
fail-open direction (ii), a `!` body carrying its OWN carrier, which is `T-19-86`.
The inheritance fact needs a body whose own command line carries nothing. Both are
now separate arms, and **direction (ii) is pinned as MEASURED rather than described
in prose**. Recorded because a probe error that looks like a finding is exactly the
shape that gets asserted by mistake. Commit `bd8be7c`.

## FOUND AND NOT FIXED — `T-19-110`, registered OPEN at `high`

**A new finding this plan made while writing region 2. It is NOT `T-19-108`.**

`scan_config`'s walk treats any word beginning with `-` as an OPTION. **Git 2.43.0
does not agree**: once the KEY has been seen, the next word is the VALUE whatever
its first byte is. Measured against real git with both config pointers at an empty
file:

```
git config alias.x -q         -> exit 0, alias.x=-q
git config alias.y --global   -> exit 0, alias.y=--global
git config core.hooksPath -c  -> exit 0, core.hooksPath=-c
```

Measured against the **BUILT BINARY**, **plan 19-02's by-name `core.hooksPath` deny
at region 2 is reachable past**:

```
exit 2 hook_bypass_blocked   git config core.hooksPath /dev/null
exit 2 hook_bypass_blocked   git config core.hooksPath -
exit 0                       git config core.hooksPath -c        <- BYPASS
exit 0                       git config core.hooksPath --        <- BYPASS
```

**Why it was NOT fixed here.** It is a gap in region 2's **operand grammar**, not
in the re-parse question this round was scoped to. Correcting it means widening
`is_write`, which moves verdicts for keys **outside this round's class with no
corpus able to fail on them** — and this phase's whole discipline is that a rule is
written against a corpus observed RED first. That corpus does not exist yet, and
this plan may not create it in the files that would hold it. The rows are
**RECORDED, never asserted**, in
`the_region_2_operand_grammar_gap_is_recorded_by_19_25_and_is_not_closed_by_it`,
beside the two spellings the deny does reach: asserting them PERMITTED would pin a
bypass as correct, and asserting them REFUSED would pin a verdict this round did
not produce — `19-22`'s failure mode.

`19-25`'s re-parse clause is **not** blind to the same gap (it reads the value
WORD), so the clause's own reach is complete. The remedy shape for a future round
is recorded in `deferred-items.md`.

## Self-Check: PASSED

- `src/envelope/policy.rs`, `src/envelope/cred.rs`,
  `tests/envelope_reparsed_value.rs` — FOUND
- `19-SECURITY.md`, `deferred-items.md` — FOUND
- commits `bd8be7c`, `6174ce7`, `b45c2a2`, `089fb18` — all FOUND in `git log`
- `AR-19-13` row count: **0**
- `tests/` numstat across all four commits: **294 insertions, 0 deletions**
- `tests/envelope_command_position.rs` / `tests/envelope_config_resolution.rs`
  diff lines: **0**
- `src/envelope/hooks.rs` diff lines: **0**
- `cred.rs` non-doc changed lines: **0**
- `policy.rs` `#[cfg(test)]` count: **1**
- `19-SECURITY.md` diff: **453 insertions, 0 deletions** (append-only)
- `Cargo.toml` / `Cargo.lock`: untouched
- working tree clean apart from the pre-existing untracked `.gsd/`

## What remains uncovered

**`T-19-86` — FIRST, and OPEN at `high`.** By explicit user scoping decision. Its
four registered rows and its persisted-alias arm still exit 0, pins green and
UNMODIFIED. **Recorded MEASURABLY WIDER than the register credits — it carries
`T-19-108`'s unclosed `!`-bodied arm — and NOT closed.** Audit 7's destructive pair
still moved a bare remote's ref in this plan's own gate run (`d7cd9d7` →
`258650e`).

- **`T-19-91`** — OPEN at `high`, arms unweakened; no decision-operand rule added,
  no denylist extended.
- **`T-19-96`** — left exactly as pinned by `19-16`.
- **`T-19-74`** — core rows frozen.
- **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by explicit
  user decision. The ONE named exception taken is `cred.rs`'s `hooks_path_env`
  limit paragraph (doc only, zero non-doc lines); `advisory.rs`, `scan.rs`,
  `config.rs`, `mod.rs` and `hooks.rs` were not opened.
- **`T-19-110`** — NEW, OPEN at `high`, recorded above and not fixed.
- **the three fail-open directions of `REPARSED_COMMAND_SECTIONS`** — uncovered by
  any automated control, by construction. The version witness is a SCHEDULE, not a
  control.
- **the `glab --host` forge cell** — carried forward UNFIXED; `glab` is not
  installed, so the callee's grammar is unconfirmed and a pin that skips is
  fail-open.
- **`policy.rs`'s stale proportional-floor comment** — documentation drift,
  recorded, not fixed; now staler, since the production half grew from 228,101 to
  261,386 bytes.
- **the `T-19-17r` bookkeeping gap** — still **OUTSTANDING**. This plan added no
  Accepted-Risks-Log row, created no `AR-19-13`, and does not apply the word
  "accepted" to it anywhere, because accepting a risk is a human decision that five
  agents have now deliberately left unmade.
- **the three documented flakes** — pre-existing, environmental, out of scope; none
  fired in this run, and none was touched.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed. `/gsd-secure-phase 19`
is NOT cleared by this plan.**
