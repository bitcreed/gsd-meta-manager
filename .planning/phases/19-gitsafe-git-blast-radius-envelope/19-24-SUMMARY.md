---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 24
subsystem: envelope
tags: [security, corpus, config-resolution, red-before-fix, T-19-108]
status: complete
requires:
  - "19-23's rules (round 8's confinement clause, non-dead and pinned here)"
  - "audit 8's measured T-19-108 rows"
provides:
  - "tests/envelope_reparsed_value.rs — the EIGHTH evidence file, RED"
  - "CONFIG_REPARSED_VALUE_CARRIERS + CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS"
  - "a SIXTH class on the EXISTING config-resolution axis"
  - "the mechanical SHELL-ALIAS FENCE"
  - "the RED handoff contract 19-25 gates against"
affects:
  - "19-25 (writes the rules; gated on this plan's recorded RED state)"
tech-stack:
  added: []
  patterns:
    - "corpus-first: the widened corpus is observed RED before any production line moves"
    - "measure against the built binary AND against the real callee before asserting"
    - "record rather than assert a verdict the plan cannot derive"
key-files:
  created:
    - tests/envelope_reparsed_value.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "A SIXTH class on the EXISTING axis, not a fifth axis — the region moved, not the axis"
  - "The shell-alias carve-out is git's own one-byte rule, fenced mechanically rather than by care"
  - "T-19-108 will close only AS SCOPED; audit 7's !-bodied pair is T-19-86 and stays open"
  - "The class-2 overlap is asserted POSITIVELY for the -c delivery and ABSENT for the persisted one"
metrics:
  duration: ~2h
  completed: 2026-09-04
actuals:
  tokens: 88000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 24: The Corpus for a Carrier Inside a Value the Guard Confined — Summary

The corpus for the REGION round 8's rule does not reach — a carrier delivered
inside a config VALUE the guard itself confined — written, measured, and committed
RED with zero `src/` hunks.

## STATED FIRST, as the plan requires

**`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-25`, or by the two
together.** `T-19-86` and `T-19-91` both remain OPEN at `high`. Only the
WRAPPER-OPERAND sub-class of `T-19-60` is closed; no unqualified "T-19-60 is
closed" appears anywhere in this plan's output.

**This plan closes NOTHING.** `T-19-105`, `T-19-106`, `T-19-108` and `T-19-109` are
all OPEN at its end. The rules are `19-25`'s.

**`T-19-108` will close only AS SCOPED, and audit 7's `!`-bodied destructive pair
STILL WORKS after this round.** The rule `19-25` writes closes the *non-shell*
alias body. It does not close
`git config alias.q '!git -c include.path=<evil> push --force origin
HEAD:refs/heads/main'` then `git q` — because a `!` body is a whole command line
handed to a governed program as DATA, which is `T-19-86`: open at `high`, out of
scope by explicit user decision, and pinned PERMITTED in two files this round may
not edit. **Re-measured against a rebuilt bare remote after `19-23` landed, that
pair still moved the ref, `d833ba0` → `9687d94`**, and the corpus asserts that it
does, so a scoped closure cannot silently take `T-19-86` on.

**`T-19-86` is recorded MEASURABLY WIDER than the register credits and is NOT
closed.**

## Commits — three, in order, ZERO `src/` hunks in each

| # | SHA | What | `src/` hunks |
|---|---|---|---|
| 1 | `0176d3c` | the EIGHTH evidence file, RED | **0** |
| 2 | `7388038` | the SIXTH class on the existing axis, RED | **0** |
| 3 | `32a3b35` | the record, the corrected `T-19-106`, the gate | **0** |

Across all three: 4 files, +3,456 / −38. **No `src/` line changed. `Cargo.toml` and
`Cargo.lock` untouched.** The 38 removed lines in `tests/envelope_wrapper_class.rs`
are, every one, a floor RAISED (`161`→`264`, `69`→`111`, `92`→`148`, `105`→`203`,
`MIN_CONFIG_RESOLUTION_CLASSES` `5`→`6`, `MIN_CONFIG_CONFINED_CARRIERS` `5`→`7`) or
a loop header widened to cover more alphabets, or a `println!` format string.
**Nothing was lowered, deleted or narrowed.** `tests/envelope_command_position.rs`
and `tests/envelope_config_resolution.rs` show ZERO diff lines.

## The gate — arithmetic STATED and CHECKED

`rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep` over a
redirected log.

| | |
|---|---|
| baseline `passed + failed` | **1680** |
| observed `passed + failed` | **1710** (1702 passed, 8 failed, 13 ignored) |
| new `#[test]` fns, counted from `git show` | 29 + 1 = **30** |
| identity | 1680 + 30 = **1710** ✓ no disagreement |

A red test RAN, so red→green leaves the total unchanged and every increase comes
only from new fns. **All FIFTEEN `envelope_*` binaries RAN.**
`cargo build` and `cargo clippy -- -D warnings` exit 0.

### Per-binary counts, all fifteen confirmed to have run

| binary | run | | binary | run |
|---|---|---|---|---|
| `envelope_advisory` | 10 | | `envelope_pr_cap` | 11 |
| `envelope_argv_deletion` | 20 | | **`envelope_reparsed_value`** | **29** (22 passed, 7 failed) |
| `envelope_callee_grammar` | 19 | | `envelope_tracer` | 6 |
| `envelope_command_position` | 18 | | `envelope_wiring` | 14 |
| `envelope_config_resolution` | 30 | | `envelope_wrapper_bypass` | 13 |
| `envelope_credential` | 6 | | **`envelope_wrapper_class`** | **41** (40 passed, 1 failed) |
| `envelope_expansion_slots` | 32 | | | |
| `envelope_hook_refusals` | 7 | | | |
| `envelope_literal_decision` | 43 | | | |

The fourteen pre-existing binaries each match audit 8's recorded count exactly.

### The RED set — `19-25`'s handoff contract, complete

Eight failures, every one a name this plan created. **No other test failed.**

```
tests/envelope_reparsed_value.rs
  after_19_25_a_reparsed_alias_value_carrying_an_indirection_is_refused
  after_19_25_a_reparsed_alias_value_reaching_the_by_name_hooks_deny_is_refused
  after_19_25_the_case_varied_and_second_carrier_spellings_are_refused_too
  after_19_25_a_persisted_alias_carrying_an_indirection_is_refused_in_every_spelling
  after_19_25_the_three_non_shell_boundary_spellings_are_refused
  after_19_25_defining_an_ordinary_non_shell_alias_is_refused_and_its_twin_is_not
  after_19_25_the_scan_order_decides_which_clause_names_a_line_carrying_both

tests/envelope_wrapper_class.rs
  a_carrier_inside_a_confined_config_value_fails_closed_in_both_deliveries
```

**No documented flake fired in this run.** Absence is not evidence they are fixed,
and one firing would not have been evidence of a new defect. The three remain
pre-existing, environmental and out of scope.

**No row expected RED came up green.** Every `after_19_25_*` test is red and every
green-today test is green.

## The SHELL-ALIAS seam, stated prominently

**Git's shell-alias rule is a ONE-BYTE fact, and a blanket `alias.*` refusal is
UNDISCHARGEABLE.** Two evidence files `19-25` may not edit pin a `!`-bodied alias
PERMITTED as a registered `T-19-86` row:

```
tests/envelope_command_position.rs:550       permits("git -c alias.p='!git push --force origin main' p")
tests/envelope_config_resolution.rs:1539-43  permits("git config alias.p \"!git push --force origin HEAD:refs/heads/main\"")
```

Measured in **nine spellings** against `git version 2.43.0`, with the envelope's own
injection as the control:

| spelling | git resolves | kind |
|---|---|---|
| control, no carrier | `/ENV_WINS` | — |
| `-c alias.a='-c include.path=<f> config --get core.hooksPath' a` | `/INCLUDE_WINS` | **K1, IN-PROCESS** |
| `-c alias.b='!git config --get core.hooksPath' b` | `/ENV_WINS` | **K2, SHELL child inherits** |
| `-c alias.g='config --get core.hooksPath !x' g` (`!` not first) | `/ENV_WINS` | in-process |
| `-c alias.d=' !git …' d` (SPACE first) | `expansion of alias 'd' failed; '' is not a git command` | refuses to expand |
| `-c alias.t='<TAB>!git …' t` | `expansion of alias 't' failed; '' is not a git command` | refuses to expand |
| `-c alias.q='"!git -c include.path=<f> …"' q` (QUOTED) | `expansion of alias 'q' failed; '!git -c include.path=<f> …' is not a git command` | **IN-PROCESS** |
| `-c alias.e='' e` (EMPTY) | `expansion of alias 'e' failed` | refuses to expand |
| `-c alias.o='-c include.path=<f>' o` (option only) | `fatal: empty alias for o` | refuses |

**The QUOTED-body row is the one that shows reading ONE BYTE is not a loophole.**
Its first byte is `"`, so a first-byte rule refuses it — and that refusal is
CORRECT: git's error names the whole *dequoted* string as not a **git** command,
i.e. its own `split_cmdline` handled it in-process and it never reached a shell.
The quoting that would smuggle a `!` past a naive check also takes the body out of
the shell path entirely.

**No spelling was found in which the first byte IS `!` and git nonetheless
re-parses in-process.** The corpus carries that as a standing assertion; a
counterexample is a FINDING, not a row.

The fence is **mechanical**, in `tests/envelope_wrapper_class.rs`: no
`CONFIG_REPARSED_VALUE_CARRIERS` entry may have a `!` first byte, a `!`-bodied
entry MUST be in `CONFIG_CONFINED_CARRIERS`, and both failure messages name both
pinning files, both line numbers, git's measured rule and the consequence.

## Every measured guard row, with its walk

Driven against the built binary, one fresh `GSD_MM_ENVELOPE_ROOT` per row, the
whole envelope directory WALKED afterwards. **Walk non-blindness proved in the same
run**: `gh pr create --title x` → exit 0 with exactly ONE
`alpha/pr-ledger.ndjson` line.

### The `-c`-delivery `T-19-108` rows — all exit 0, walk EMPTY, all asserted exit 2 `envelope_assertion_failed`

```
git -c alias.q="-c include.path=/tmp/evil.cfg push --force origin HEAD:refs/heads/main" q   <- ONE line
git -c alias.z="-c core.hooksPath=/dev/null push origin refs/heads/gsd-auto/alpha/w" z      <- reaches 19-02's deny
git -c alias.q="-c include.path=/tmp/evil.cfg status" q
git -c alias.q="-c include.path=/tmp/evil.cfg commit -m x" q
git -c ALIAS.q="-c include.path=/tmp/evil.cfg status" q                                     <- the case row
git --config-env=alias.q=EVILBODY status                                                    <- the second carrier
```

### The PERSISTED-delivery rows — all exit 0, walk EMPTY, all asserted exit 2 `envelope_assertion_failed`

`git config alias.p '<non-shell body>'` and its `--global`, `--worktree`, `--file`,
`--add` and `--replace-all` spellings, plus
`git config alias.z '-c core.hooksPath=/dev/null status'`.

### The paired discriminators — asserted UNCHANGED

| exit | reason | command |
|---|---|---|
| 2 | `envelope_assertion_failed` | `git -c include.path=/tmp/evil.cfg push --force origin main` |
| 2 | `hook_bypass_blocked` | `git -c core.hooksPath=/dev/null status` |
| 2 | `hook_bypass_blocked` | `git config core.hooksPath /dev/null` |

These are what make this a gap in **REACH**, not in mechanism: the same two keys,
delivered in a region the guard DOES read, already refuse at their own identifiers.

### The DISCRIMINATION controls — this round's `--signed no`, exit 0 before AND after

`git -c aliasx.q="…" q` and `git -c notalias.q="…" q`. A rule written as
`key.starts_with("alias")` turns the first red; `key.contains("alias")` turns the
second red; only a SECTION comparison keeps both green. Beside them the dotless
fence `git -c a=b status` / `git -c a=b version`, behind which
`CALLEE_KNOWN_LEADING_PREFIX` splices round 7's whole generative property.

### The DISCLOSED OVER-REFUSAL family, every row beside its permitted twin

All four measured exit 0 today, all asserted exit 2 after; all four twins asserted
exit 0 before AND after.

| refused after `19-25` (a DEFINITION) | permitted twin (does the same work) |
|---|---|
| `git -c alias.st=status st` | `git status` |
| `git -c alias.lg="log --oneline" lg` | `git log --oneline` |
| `git -c alias.co=checkout co` | `git checkout` |
| `git config alias.co checkout` | run the command the alias would have run |

**Ordinary aliases do NOT keep working, and saying so plainly is the point.**

### The INVOCATION rows — UNCHANGED at exit 0, before and after

`git p`, `git co`, `git st`, `git lg`, `git z`, `git q`. The guard is stateless and
argv-only and cannot see an alias it did not watch being defined. **The round's
whole cost in one sentence: defining a non-shell alias is refused; using one is
not.**

### The two ORDERING rows — both `hook_bypass_blocked` today, pinned at DIFFERENT identifiers

```
alias FIRST : -c alias.q="…" -c core.hooksPath=/dev/null push --force origin main
              -> asserted envelope_assertion_failed   RED (verdict does not move; the IDENTIFIER does)
hooks FIRST : -c core.hooksPath=/dev/null -c alias.q="…" push --force origin main
              -> asserted UNCHANGED at hook_bypass_blocked
```

Together they are the mechanical proof that the clause is raised inside the ONE
left-to-right walk and not in a second pass.

### The two rows RECORDED rather than asserted

Measured, printed, and **NOT** written as assertions — asserting merely that they
are refused is also forbidden. `19-22` asserted such a row against its own comment,
its own SUMMARY and its own plan-check, and it halted `19-23` mid-plan.

| row | measured | why it is left for `19-25` |
|---|---|---|
| `git --config-env alias.q=BODYVAR status` (SEPARATE-WORD) | exit 0, walk EMPTY | the value half is an environment variable NAME, so whether `19-25` can read a first byte at all is a design decision. Real git DOES resolve it (`/INCLUDE_WINS`), so the harm is real; only the identifier is undeliverable |
| `git -c alias.q status` (no `=` at all) | exit 0, walk EMPTY | `config_key_of` returns the whole token, so there is NO value half. Real git resolves the control value (`/ENV_WINS`) — **no harm on git's side** |

### The `T-19-86` rows — exit 0, asserted UNCHANGED, recorded WIDER, NOT closed

`git submodule foreach git push --force origin main`,
`git rebase -x "git push --force origin main" HEAD~3`,
`git bisect run sh -c "git push --force origin main"`,
`git -c alias.p='!git push --force origin main' p`, and the persisted-alias arm.

## The K1/K2/K3 enumeration — measured, not argued

- **K1 — re-parsed as a GIT command line, IN-PROCESS, including its leading
  options.** `alias.<name>` with a non-`!` body. **Measurement says it is the ONLY
  member**, and it is the whole of `T-19-108`.
- **K2 — re-parsed as a SHELL command line, run as a CHILD that INHERITS the
  injection. INERT for layer 3, with the MECHANISM recorded rather than the verdict
  alone.** FOUR representatives measured with the child's own environment DUMPED —
  a `!` alias body, `diff.external`, `credential.helper` (via `git credential fill`,
  which needs stdin to invoke the child at all) and `filter.<n>.clean` — every one
  printing `CHILD_ENV COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS` and
  `CHILD_RESOLVES /ENV_WINS`. **No K2 member was found that does not inherit**; one
  would have been a FINDING.
- **K3 — re-parsed as a FILE PATH spliced at the directive's precedence.**
  `include.path`, `includeIf.<cond>.path`. Closed by `19-23`.

## The PERSISTENCE ASYMMETRY with `include.path`

Audit 8's INERT include rows are carried forward as ESTABLISHED and were also
re-confirmed here. Under the envelope's own config posture — `GIT_CONFIG_GLOBAL`
and `GIT_CONFIG_SYSTEM` both pointed at a generated helper-free file, the triplet
injected:

```
git config --global alias.g '-c include.path=<f> config --get core.hooksPath'; git g -> /INCLUDE_WINS   LIVE
git config          alias.l '-c include.path=<f> config --get core.hooksPath'; git l -> /INCLUDE_WINS   LIVE
git config          include.path <f>                                                  -> /ENV_WINS      INERT
git config --global include.path <f>                                                  -> /ENV_WINS      INERT
```

**An include must WIN a precedence contest and loses; an alias only has to EXIST.**
Both persistence levels are live, including the one the envelope itself controls.
**A clause for `git config include.path` would be INERT and must NOT be added** —
the likeliest way `19-25` wastes a rule. The corpus pins
`git config include.path /tmp/evil.cfg` PERMITTED so that adding it lands red.

## DEPTH and QUOTING

```
depth 2: -c alias.d2='-c "alias.inner=-c include.path=<f> config --get core.hooksPath" inner' -> /INCLUDE_WINS
depth 3: -c alias.d3='-c "alias.d2=-c \"alias.inner=…\" inner" d2'                            -> /INCLUDE_WINS
dequote: -c alias.m='config --get "core.hooksPath"' m                                         -> /ENV_WINS (RESOLVES)
control: git config --get '"core.hooksPath"'                                                  -> error: invalid key
```

**Depth 3 is the row that matters**: the recursion is not a depth-2 curiosity, so
any stated depth bound is a residue an attacker reaches by adding one more nesting
level. Git splits an alias body with its OWN `split_cmdline` rules, not the
shell's — the dequoting row resolves where the literally-quoted key is an
`invalid key` error.

**A probe error recorded honestly.** The first depth-2 probe reported an expansion
failure. That was an error in the PROBE — the inner carrier was unquoted inside the
body, so git's own whitespace splitting broke it apart — not a contradiction of the
plan. Re-run with the inner carrier quoted, both depths reproduce at
`/INCLUDE_WINS`. Recorded because a probe error that looks like a finding is
exactly the shape that gets asserted by mistake.

## The bare-remote fixture — rebuilt, with a CONTROL beside every leg

Not inherited by citation; claims in this codebase about what outranks the
injection have been wrong twice. Local bare upstream, offline, `core.hooksPath`
delivered exactly as `cred::hooks_path_env` emits it.

| leg | command | result | ref before → after |
|---|---|---|---|
| 1 CONTROL | plain in-namespace push | REFUSED by `pre-push` | `0084939` → `0084939` **unmoved** |
| 2 CONTROL | persisted NON-SHELL alias **without** the include | REFUSED by `pre-push` | `0084939` → `0084939` **unmoved** |
| 3 CARRIER | `git config alias.p '-c include.path=<evil> push …'` then `git p` | COMPLETED | `0084939` → **`4a77396`** |
| 4 CARRIER | `-c alias.q='-c include.path=<evil> push …' q`, ONE line | COMPLETED | `4a77396` → **`d833ba0`** |
| 5 CONTROL | `commit` | REFUSED by `pre-commit` | HEAD unmoved |
| 5 CARRIER | `-c include.path=<evil> commit` | COMPLETED | HEAD moved |
| **T-19-86** | `!`-bodied pair — **NOT CLOSED** | COMPLETED | `d833ba0` → **`9687d94`** |

**Leg 2 is the sharpest control in the fixture**: the same alias mechanism with no
carrier is still caught by the hook, which isolates the CARRIER from the ALIAS.
Without it the harm would be mis-attributed as "aliases evade hooks". **Every leg
reproduced; none failed to reproduce.**

## The SIXTH class — the axis did not move, the REGION did

`CONFIG_RESOLUTION_CLASSES` gains *a carrier delivered inside a config VALUE the
guard confines*, with `MIN_CONFIG_RESOLUTION_CLASSES` raised 5 → 6.
`UNREADABLE_CLASSES`, `DELETION_CLASSES` and `CALLEE_GRAMMAR_CLASSES`, their
predicates, their degenerate-proofing and every one of their floors are
byte-identical. **A fifth axis would have modelled a REGION as if it were a STAGE
and lost the distinction audit 8 drew.**

**The two deliveries are open for two DIFFERENT reasons, asserted mechanically:**

- the `-c` delivery **IS** read by round 8's region and **IS** confined — `alias.q`
  correctly answers `false` to `config_key_names_an_indirection_section`, so class 2
  draws it. **That overlap IS `T-19-108`**, and it is asserted POSITIVELY so a later
  round cannot "tidy" class 2 to exclude it and erase audit 8's finding;
- the PERSISTED delivery is **not read by that region at all** — `config` is the
  verb and `scan_leading` stops there — so class 2 correctly does NOT draw it.

**A rule written only inside `scan_leading` closes only the first delivery.**

Floors re-derived and stated as exact equalities, **all confirmed by the counting
test on the first run**: 264 cases (111 refused / 148 permitted / 5 persisted) over
21 slots, from 7 confined / 5 indirection / 5 reparsed-value / 3 environment
entries; per class **70 / 203 / 21 / 28 / 28 / 75**.

Two `!`-bodied entries were added to `CONFIG_CONFINED_CARRIERS` — measured
verdict-preserving over all 28 generated cases with **ZERO mismatches** — which is
what makes the corpus able to fail on a rule that refuses them.

`shell_words` was added beside `config_prefix_split`'s whitespace split, because the
one-byte fence turns on the difference between a shell-quoted value and a value
whose first byte is literally a quote. **`config_assignments` is byte-identical**;
the five existing predicates are untouched.

## The design question `19-25` must answer

**(a) scan a confined value git will re-parse, recursively with a stated depth
bound, or (b) treat any assignment whose key names a re-parsed value as unbounded
and refuse?** The costing is `19-25`'s; the discrimination is this plan's. **This
corpus turns RED on:**

- a prefix or substring match on `alias` — red at `aliasx.` and `notalias.`;
- a blanket `alias.*` refusal including `!` bodies — red at both `T-19-86` rows and
  mechanically red at the SHELL-ALIAS FENCE;
- a refusal of a key it cannot decompose into a section — red at `-c a=b`, and
  through `CALLEE_KNOWN_LEADING_PREFIX` at round 7's whole generative property;
- a rule written only inside `scan_leading` — red at all five persisted rows;
- a clause raised in a SECOND PASS — red at the two ordering pins;
- a blanket refusal of everything spelled `-c` — red at the invariance arm and the
  seven-row permitted half;
- an INERT `git config include.path` clause — red at the pinned permit.

## The mechanism pins — six, carried forward and GREEN

Round 5's literalness bit non-vacuous; round 6's deletion model non-dead with its
over-deletion control; `policy::is_separator(">") == false` with `SEPARATORS`
byte-identical; round 7's fail-closed callee grammar non-dead in all four
directions; **round 8's confinement clause non-dead** with its two discrimination
controls; and Rule B's severed-head geometry in both directions. All re-asserted in
the new file over the same public functions, not moved and not edited in place.

## The byte floors and the stripper's three protections

| | measured | floor |
|---|---|---|
| `policy.rs` | raw 372,920 / production **228,101** | `POLICY_MIN_PRODUCTION_BYTES = 40_000`, proportional `>= 180_000` (**78.9%**) |
| `hooks.rs` | raw 99,753 / production **74,358** | `HOOKS_MIN_PRODUCTION_BYTES = 20_000` |
| deep anchor | `fn forbidden_repo_path` at production line **4,592** of **4,613** | present |
| sentinel | exactly **ONE** `#[cfg(test)]`, at line **4,614** | count asserted |

All three protections keep their present strictness; both byte floors keep their
values.

## `T-19-106` — the mis-closure, CORRECTED from CLOSED to OPEN at `medium`

`deferred-items.md` marked it CLOSED at round 8; audit 8 re-opened it because
`GH_API_VALUE_OPTS` is a pure `gh api` constant, **`gh` 2.45.0 IS installed**, and
the stated reason for leaving it unpinned is a fact about a different callee. The
entry is corrected; the rule is `19-25`'s.

Re-measured with an **ENDPOINT-LESS** probe (`gh api <opt>`), so **no row touches
the network**. All seventeen entries answer `flag needs an argument`. Two negative
controls: `gh api --bogus-opt` → `unknown flag: --bogus-opt`, and **`gh api
--paginate` → `accepts 1 arg(s), received 0`** — recorded as the measured string
rather than a paraphrase, because it is a POSITIONAL error and not a flag-level
one, which is exactly what makes it the right control: the flag was ACCEPTED and
consumed NO value. **The endpoint-bearing `gh api repos/o/r --paginate` makes a
REAL HTTP REQUEST**, so a pin written that way would be non-hermetic and fail open
without network or auth.

**A correction to audit 8's own suggestion, measured.** Removing `--hostname` from
`FORGE_VALUE_OPTS` would be a **REGRESSION in the under-counting direction**:
`glab --hostname gitlab.com mr create --title x` leaves exactly ONE ledger line
today, while `glab --host …` — not in that constant — leaves ZERO. **Disposition:
"record why", not "remove".**

## The fail-open residue's missing revisit condition

Recorded as audit 8 recorded it: the admission is complete and correctly unclaimed
in five places, the residual is acceptable and properly bounded, and the single gap
is that its only control is a human reading a future git's release notes and
**nothing schedules that**. **The witness `19-25` adds is a SCHEDULE and not a
CONTROL** — it says WHEN to look and cannot say WHAT changed, so the residues stay
uncovered by any automated control and audit 8's judgement of them is unchanged.
**This plan invents no acceptance and adds no `AR-` row.**

## `T-19-17r` — OUTSTANDING, and this plan did NOT accept it

`19-17-SUMMARY.md` calls it "accepted". Audits 5, 6, 7 and 8 all confirmed the
measurement and both pins and **all four deliberately declined to make the
acceptance**. The Accepted Risks Log runs `AR-19-01` … `AR-19-12`; **there is no
`AR-19-13` row (verified: count 0)** and no register row. **This plan did not make
the acceptance, added no Accepted-Risks-Log row, and does not apply the word
"accepted" to `T-19-17r` anywhere in its artifacts**, because accepting a risk is a
human decision. The next round either adds the log row or drops the word; this plan
does neither.

## Known Stubs

None. This plan adds test evidence and records only; every assertion it writes is
either measured green today or measured red today at a derived post-fix verdict,
and the two undeliverable rows are recorded rather than stubbed.

## Deviations from Plan

**1. [Rule 1 — corpus correctness] The class-2 overlap is asserted per-delivery
rather than uniformly.** The plan's task-2 wording asked for the new class to be
satisfied by both deliveries "and by none of the five existing classes". Measured,
the `-c` delivery **does** satisfy class 2 — and that is the finding, not a defect:
round 8's clause CONFINES `alias.q` and lets it through. Asserting `!class2` there
would have been a false statement about the mechanism. The persisted delivery does
**not** satisfy class 2, for a different and equally load-bearing reason: `config`
is the verb, so round 8's region never sees it. Both facts are now asserted
explicitly with their reasons, which is strictly stronger than the uniform
assertion and preserves audit 8's finding in the corpus. Commit `7388038`.

**2. [Rule 1 — evidence legibility] The fence's RED rows were split into their own
test.** The plan put the three non-shell boundary spellings (the `!`-not-first row,
the quoted body, the TAB) inside the same test as the fence's PERMITTED half. That
made the whole fence test red, so a genuine fence breakage would have been
indistinguishable from the expected red. They now live in
`after_19_25_the_three_non_shell_boundary_spellings_are_refused`, leaving
`a_shell_bodied_alias_stays_permitted_because_gits_rule_is_the_first_byte` GREEN
today — so its passing IS the evidence the fence held. Every row the plan named is
still asserted. Commit `0176d3c`.

**3. [Rule 1 — probe correctness] Two K2 probes were corrected, not dropped.** The
`credential.helper` representative needs **stdin** (`git credential fill`) to make
git invoke the child at all; a probe that merely read the key would print the value
and measure nothing. It is now driven with a piped stdin. `core.pager` and
`core.editor` were **not** exercised in this harness and are therefore **not
claimed** — four representatives were measured, above the plan's floor of three.

## Self-Check: PASSED

- `tests/envelope_reparsed_value.rs` — FOUND
- `tests/envelope_wrapper_class.rs` — FOUND
- `19-SECURITY.md`, `deferred-items.md` — FOUND
- commits `0176d3c`, `7388038`, `32a3b35` — all FOUND in `git log`
- `src/` hunks in each: 0, 0, 0
- `AR-19-13` row count: 0
- `tests/envelope_command_position.rs` / `tests/envelope_config_resolution.rs` diff
  lines: 0
- working tree clean apart from the pre-existing untracked `.gsd/`

## What remains uncovered

**`T-19-86` — FIRST, and open at `high`.** By explicit user scoping decision. Four
registered rows plus the persisted-alias arm still at exit 0, pins green and
UNMODIFIED. **Recorded MEASURABLY WIDER than the register credits and NOT closed**:
the layer-3 catch three documents attribute to closing `T-19-103` is ABSENT when
the alias body carries a carrier of its own, and audit 7's `!`-bodied destructive
pair still moved a bare remote's ref (`d833ba0` → `9687d94`) after `19-23` landed.

- **`T-19-91`** — OPEN at `high`, arms unweakened (`git reflog $S`,
  `git reflog show $S`, `git symbolic-ref $S` at exit 0 with no second carrier), no
  decision-operand rule added, no denylist extended.
- **`T-19-96`** — a glob in a push flag, left exactly as pinned by `19-16`.
- **`T-19-74`** — core rows frozen.
- **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by explicit
  user decision. `cred.rs`, `advisory.rs`, `scan.rs`, `config.rs`, `mod.rs` and
  `hooks.rs` were not touched.
- **`T-19-105`, `T-19-106`, `T-19-108`, `T-19-109`** — all OPEN; the rules are
  `19-25`'s.
- **the `glab --host` forge cell** — carried forward UNFIXED. `command -v glab` was
  re-run and found nothing, so the callee's grammar is unconfirmed and it is NOT
  claimed as a live bypass.
- **`policy.rs:6644`'s stale proportional-floor comment** — documentation drift,
  recorded, not fixed (this plan carries zero `src/` hunks).
- **the three documented flakes** — pre-existing, environmental, out of scope; none
  fired this run.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed. `/gsd-secure-phase 19`
is NOT cleared by this plan, and will not be cleared by `19-25` either.**
