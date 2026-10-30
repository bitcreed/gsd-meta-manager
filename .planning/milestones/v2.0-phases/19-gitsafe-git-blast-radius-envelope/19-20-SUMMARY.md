---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 20
subsystem: envelope-guard-evidence
tags: [security, corpus, red-before-fix, callee-grammar, git-option-grammar]
status: complete
requires: [19-19]
provides:
  - "tests/envelope_callee_grammar.rs — the SIXTH evidence file, RED"
  - "CALLEE_GRAMMAR_CLASSES — a THIRD named axis in tests/envelope_wrapper_class.rs"
  - "POLICY_MIN_PRODUCTION_BYTES / HOOKS_MIN_PRODUCTION_BYTES — the anti-vacuity control re-expressed"
  - "the exact handoff number 1631 and the 7-name RED set for 19-21 to gate against"
affects: [19-21]
tech-stack:
  added: []
  patterns:
    - "corpus written and observed RED before the rule, for the fourth consecutive two-plan handoff"
    - "post-fix verdicts asserted where derivable; undeliverable rows recorded, never asserted"
    - "class predicates read a table measured two-sided against the real callee binary"
key-files:
  created:
    - tests/envelope_callee_grammar.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The callee's grammar is a THIRD axis, not more entries in either shell axis"
  - "Assert the POST-fix verdict wherever derivable, so no body needs replacing and 19-21 needs no exception"
  - "The class-5 alphabet is exactly `--`; the class-5 PREDICATE still recognises a bare `-`"
  - "GIT_GLOBAL_UNKNOWN_OPTIONS is a separate alphabet, kept out of the invariance arm"
  - "The anti-vacuity ratio floor is REPLACED by absolute per-file byte floors, not lowered"
metrics:
  duration: "one session (paused and resumed once)"
  completed: 2026-09-03
actuals:
  tokens: 76000
  tasks: 3
  commits: 3
requirements: [SAFE-02, SAFE-05, SAFE-06]
---

# Phase 19 Plan 20: The Corpus for the Callee's Grammar Summary

**Audit 6 verified the command-line-to-argv boundary CLOSED; this round writes the
corpus for the layer above it — whose grammar decides which arriving word is the
verb — and observes it RED before a line of the rule exists.**

---

## FIRST: this plan closes nothing, and `/gsd-secure-phase 19` is NOT cleared

**`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-21`, or by the two
together.**

- **`T-19-86` remains OPEN at `high`** — by explicit user scoping decision. Its
  four rows still exit 0 and its pin is green and unmodified.
- **`T-19-91` remains OPEN at `high`** — arms unweakened: `git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` at exit 0 with **no second
  carrier**, and bare `git push $REF` at exit 0 in the in-namespace configuration.
- `T-19-100`, `T-19-101` and `T-19-102` all stay OPEN at this plan's end.
  `T-19-101` is closed only when `19-21`'s rule is certified by the corpus written
  here, because a corpus is evidence about a control and there is no control yet.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` are both sub-classes of it and both remain open at `high`.

---

## The three commits, in order — ZERO `src/` hunks in each

| # | SHA | What | `src/` hunks |
|---|---|---|---|
| 1 | `a5404f7` | `test(19-20)`: the sixth evidence file — the callee's grammar, RED before the rule | **0** |
| 2 | `fcdb9f7` | `test(19-20)`: `CALLEE_GRAMMAR_CLASSES` — a third axis, and the ratio floor replaced | **0** |
| 3 | `f5f53bc` | `docs(19-20)`: the plan-19-20 record, the registrations, and the handoff gate | **0** |

Verified per commit with `git show --stat HEAD | grep -c '^ src/'` → `0`. Across
all three, `git diff --stat HEAD~3..HEAD` touches exactly the four files in
`files_modified` and **neither `Cargo.toml` nor `Cargo.lock`** (`T-19-SC`).

---

## The gate arithmetic, STATED and CHECKED

Command: `rtk proxy cargo test --no-fail-fast`, counts read with
`rtk proxy grep 'test result:'` over a redirected log. A plain `cargo test`
fail-fasts at `driver_reattach` and `envelope_*` sorts after `driver_*`, so an
unqualified run never executes a single envelope suite; and the RTK hook strips
exactly the `test result:` lines any count is read from, which is how `19-19`
first read 1364 instead of 1605 (D-34).

| | passed | failed | ignored | `passed + failed` |
|---|---|---|---|---|
| **Before** (`c21c13f`) | 1610 | 0 | 13 | **1610** |
| **After** (`f5f53bc`) | 1623 | 8 | 13 | **1631** |

**The identity, checked against `git show` rather than assumed:**

```
new #[test] fns in a5404f7 (git show a5404f7 -- tests/ | grep -c '^+#\[test\]') = 16
new #[test] fns in fcdb9f7 (git show fcdb9f7 -- tests/ | grep -c '^+#\[test\]') =  5
                                                                    TOTAL NEW =  21

1610 + 21 = 1631   ==   the observed total.  The identity HOLDS exactly.
```

A red test **RAN**, so red→green leaves the total unchanged and every increase
comes ONLY from new `#[test]` fns. **`19-21` gates on strictly exceeding 1631**
with the seven carry-forward RED rows below turned green and named.

`cargo build` and `cargo clippy -- -D warnings` both exit 0. `cargo clippy
--tests` is NOT the gate — it already fails at base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs`, which are out of scope and were not
touched.

---

## THE COMPLETE RED SET — `19-21`'s handoff contract

Seven names. Every one is expected to fail against the pre-fix tree and every one
must be GREEN after `19-21`.

**`tests/envelope_callee_grammar.rs` (5):**

1. `after_19_21_a_leading_git_option_that_consumes_its_value_no_longer_hides_the_verb`
2. `after_19_21_the_in_namespace_push_with_a_value_taking_flag_is_no_longer_falsely_refused`
3. `after_19_21_the_stale_and_bundled_planning_cells_are_refused`
4. `after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins`
5. `after_19_21_the_composition_rows_are_refused_which_proves_rounds_5_and_6_load_bearing`

**`tests/envelope_wrapper_class.rs` (2):**

6. `a_leading_option_whose_grammar_the_guard_does_not_know_is_not_a_verb`
7. `an_option_the_installed_git_rejects_fails_closed_on_every_base`

**The eighth failure in the gate run is a documented flake and is out of scope:**
`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`
(`tests/driver_reattach.rs`). The other two documented flakes —
`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
and the `envelope_tracer` `ExecutableFileBusy` stub-write race carried in
`deferred-items.md` since 19-07 — did not fire in this run. **None of the three was
fixed, edited or worked around**, and their absence from a run is not a
regression.

**No row came up green when I expected red.** Every audit-6 row reproduced at its
recorded verdict and none failed to reproduce.

---

## Per-binary counts — all THIRTEEN `envelope_*` binaries RAN

Twelve audit 6 observed, plus `envelope_callee_grammar` this plan adds. A run
reporting twelve is a run in which this round's own evidence file did not execute.

| Binary | passed | failed | ignored | ran |
|---|---|---|---|---|
| `envelope_advisory` | 10 | 0 | 0 | ✔ |
| `envelope_argv_deletion` | 20 | 0 | 0 | ✔ |
| **`envelope_callee_grammar`** | **11** | **5** | 0 | ✔ (NEW) |
| `envelope_command_position` | 18 | 0 | 0 | ✔ |
| `envelope_credential` | 6 | 0 | 0 | ✔ |
| `envelope_expansion_slots` | 32 | 0 | 0 | ✔ |
| `envelope_hook_refusals` | 7 | 0 | 0 | ✔ |
| `envelope_literal_decision` | 43 | 0 | 0 | ✔ |
| `envelope_pr_cap` | 11 | 0 | 0 | ✔ |
| `envelope_tracer` | 6 | 0 | 0 | ✔ |
| `envelope_wiring` | 14 | 0 | 0 | ✔ |
| `envelope_wrapper_bypass` | 13 | 0 | 0 | ✔ |
| **`envelope_wrapper_class`** | **32** | **2** | 0 | ✔ |

`--test envelope_argv_deletion`, `--test envelope_literal_decision`,
`--test envelope_expansion_slots` and `--test envelope_command_position` are
**FULLY GREEN and byte-identical** — rounds 4, 5 and 6's evidence stays
attributable to the round that produced it, which is why this plan created a
SIXTH file rather than appending to any of them.

---

## Every measured row, with its exit code, reason identifier and walk

All measured at base `c21c13f` against `./target/debug/gsd-meta-manager`, **one
fresh `GSD_MM_ENVELOPE_ROOT` per row, the whole root walked afterwards**, before
any of them was written as an assertion.

### The `T-19-100` rows

| Command | exit | reason id | walk |
|---|---|---|---|
| `git --attr-source HEAD push --force origin main` | 0 | (a permit answers nothing) | empty |
| `git --attr-source HEAD stash` | 0 | — | empty |
| `git --attr-source HEAD update-ref -d refs/heads/main` | 0 | — | empty |
| `git --attr-source HEAD config core.hooksPath /tmp/x` | 0 | — | empty |
| `git --attr-source HEAD reflog delete HEAD@{0}` | 0 | — | empty |
| `git --attr-source HEAD symbolic-ref HEAD refs/heads/x` | 0 | — | empty |
| `git --shallow-file /tmp/s push --force origin main` | 0 | — | empty |
| `git --shallow-file /tmp/s stash` | 0 | — | empty |
| `git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main` | 0 | — | empty |
| **CONTROL** `git --attr-source=HEAD push --force origin main` | 2 | `force_push_blocked` | empty |

Four have **no second carrier at all** — `stash`, `update-ref -d`, `reflog delete`
and `config core.hooksPath`, where disarming the hook IS the loss of the carrier.

**The post-consumption argv each derivation names was itself MEASURED**, in
section 0 of the new file: `git push --force origin main`, `git stash`,
`git update-ref -d refs/heads/main`, `git reflog delete HEAD@{0}` and
`git symbolic-ref HEAD refs/heads/x` at exit 2 `force_push_blocked`;
`git config core.hooksPath /tmp/x` and
`git -c core.hooksPath=/dev/null push --force origin main` at exit 2
`hook_bypass_blocked`. Derivations name observed verdicts, not described ones.

### `T-19-102` and its twins (fixture on `refs/heads/gsd-auto/alpha/w`, bare upstream)

| Command | exit | reason id |
|---|---|---|
| `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w` | **2** | `push_outside_namespace` |
| `git push origin refs/heads/gsd-auto/alpha/w` (twin) | 0 | — |
| `git push --signed no origin refs/heads/gsd-auto/alpha/w` | **2** | `push_outside_namespace` — **CORRECT, pinned REFUSED** |
| `git push -o ci.skip origin refs/heads/gsd-auto/alpha/w` | 0 | — |
| `git push --push-option ci.skip origin refs/heads/gsd-auto/alpha/w` | 0 | — |
| `git push --recurse-submodules=on-demand origin refs/heads/gsd-auto/alpha/w` | 0 | — |

The false refusal is **asserted at its POST-fix exit 0** — red now, green after,
no body for `19-21` to replace, and **no replacement exception granted or
needed**. The measured pre-fix verdict lives in the doc comment as prose, where
the evidence belongs.

### The twelve planning cells

Provenance caveat `19-14`, `19-16` and `19-18` established: **found while
planning, not by an audit.**

| Cell | exit | reason id |
|---|---|---|
| `git --super-prefix push --force origin main` | 0 | — |
| `git --super-prefix x push --force origin main` (twin) | 2 | `force_push_blocked` |
| `git -c a=b --attr-source HEAD push --force origin main` | 0 | — |
| `git >/dev/null --attr-source HEAD push --force origin main` | 0 | — |
| `git --attr-source HEAD >/dev/null push --force origin main` | 0 | — |
| ``git --attr-so`\`+NL+`urce HEAD push --force origin main`` | 0 | — |
| `bash -lc "git --attr-source HEAD push --force origin main"` | 0 | — |
| `echo hi && git --attr-source HEAD push --force origin main` | 0 | — |
| `git -pc user.name=x status` | 0 | — |
| `git - push --force origin main` | 0 | — |
| `git -C/tmp push --force origin main` | 2 | `force_push_blocked` |
| `git -- push --force origin main` | 2 | `force_push_blocked` |
| `git -c core.hooksPath=/dev/null --attr-source HEAD push` | 2 | `hook_bypass_blocked` |

All walks empty. **Two are LABELLED mis-indexes of commands real git does not
run, never live bypasses**: `git -pc user.name=x status` (`unknown option: -pc`)
and `git - push --force origin main` (`unknown option: -`). `--super-prefix` is
the same — it is not live only because git rejects the option; a stale entry for
an option git ACCEPTS would be one.

`git - push --force origin main` is additionally **pinned at exit 0 AFTER the
fix**, because `scan_leading` breaks on `-` before any option check and `19-21` is
forbidden to widen the rule into non-`-`-prefixed words.

### The PERMITTED half — twelve ordinary invocations, all exit 0, empty walks

`git --no-pager status`, `git --no-pager log --oneline`,
`git -c user.name="$NAME" commit -m x`, `git --git-dir=/tmp/g status`,
`git -C /tmp status`, `git --bare status`, `git --literal-pathspecs status`,
`git --no-optional-locks status`, `git --exec-path status`, `git --version`,
`git --attr-source HEAD status`, `git --shallow-file /tmp/s log --oneline`.

**`git --no-pager status` is this axis's `ls {git,svn}-repo`** — the row that
tells a grammar MODEL apart from a blanket refusal of anything beginning with
`-`, which is the shape this fix is one wrong step away from.

**The walk's non-blindness control:** `gh pr create --title x` → exit 0 with
**exactly ONE** ledger line, walked listing `alpha/pr-ledger.ndjson (103 bytes)`.
That is what makes an empty walk elsewhere mean something.

### The COST rows, each beside its permitted twin

| Command | exit today | asserted after |
|---|---|---|
| `git --bogus-opt status` | 0 | 2 `envelope_assertion_failed` |
| `git --no-advice status` | 0 | 2 `envelope_assertion_failed` |
| `git --no-lazy-fetch status` | 0 | 2 `envelope_assertion_failed` |
| `git --super-prefix x status` | 0 | 2 `envelope_assertion_failed` |
| `git status` (twin) | 0 | 0 |
| `git log --oneline` (twin) | 0 | 0 |

### The COMPOSITION rows — rounds 5 and 6 are load-bearing HERE

All five at exit 0 today, empty walks, asserted exit 2 `force_push_blocked` after:
`git >/dev/null --attr-source HEAD push …`,
`git --attr-source HEAD >/dev/null push …`, the continuation-inside-the-option-name
row, `bash -lc "…"`, and `echo hi && …`. **The continuation row is the pin that
turns red if round 6's deletion model is ever removed, whatever this round's rule
does** — the `\`+NL must be deleted before `--attr-source` is even spelled.

---

## The two-sided real-git probe table (git version 2.43.0)

`git <opt> version` (1W) versus `git <opt> XVALUE version` (2W).

| Option | 1W | 2W | Classification |
|---|---|---|---|
| `-c`, `--git-dir`, `--work-tree`, `--namespace` | usage | `git version 2.43.0` | consumes a separate word |
| **`--attr-source`** | usage | `git version 2.43.0` | **consumes a separate word — ABSENT from `GIT_GLOBAL_VALUE_OPTS`** |
| **`--shallow-file`** | usage | `git version 2.43.0` | **consumes a separate word — ABSENT from `GIT_GLOBAL_VALUE_OPTS`** |
| `-C` | `fatal: cannot change to 'version'` | `git version 2.43.0` | consumes a separate word (variant probe: needs a real directory; the 1W error IS the proof) |
| `--config-env` | `fatal: invalid config format: version` | `…: XVALUE` | consumes a separate word (variant probe: needs `KEY=ENVVAR`; the error names the word it swallowed) |
| `--no-pager`, `-p`, `--paginate`, `-P`, `--bare`, `--no-replace-objects`, `--literal-pathspecs`, `--glob-pathspecs`, `--noglob-pathspecs`, `--icase-pathspecs`, `--no-optional-locks` | `git version 2.43.0` | `git: 'XVALUE' is not a git command.` | self-contained |
| `--exec-path` | `/usr/lib/git-core` | identical | terminating |
| `--html-path` | `/usr/share/doc/git/html` | identical | terminating |
| `--man-path` | `/usr/share/man` | identical | terminating |
| `--info-path` | `/usr/share/info` | identical | terminating |
| `--version` | `git version 2.43.0` | identical | terminating |
| **`--super-prefix`** | **`unknown option: --super-prefix`** | same | **NOT ACCEPTED — yet CARRIED by `GIT_GLOBAL_VALUE_OPTS`** |
| **`--no-lazy-fetch`** | **`unknown option: --no-lazy-fetch`** | same | **NOT ACCEPTED (a real option in later releases)** |
| **`--no-advice`** | **`unknown option: --no-advice`** | same | **NOT ACCEPTED (a real option in later releases)** |
| `--bogus-opt` | `unknown option: --bogus-opt` | same | not accepted (negative control) |
| `--help`, `-h` | — | `No manual entry for gitXVALUE` | **UNPROBED** — no probe of this shape can classify them |

**The rejections are the point: the list is wrong in BOTH directions, which is the
evidence the LIST is the defect rather than a missing row.**

Two further measured facts:

- **`git --shallow-file=/tmp/s version` answers `unknown option:
  --shallow-file=/tmp/s`**, while `--attr-source=`, `--git-dir=`, `--namespace=`
  and `--work-tree=` all reach the verb; `--no-pager=1` and `--bare=1` are
  rejected outright. An attached spelling is therefore **always *self-contained*
  and not always *accepted***.
- **git 2.43.0 accepts NO short-option bundling**: `-pc user.name=x`, `-C/tmp`,
  `-pP` and a bare `-` all print `unknown option:`.

---

## The three undeliverable rows — RECORDED, never asserted

All three at exit 2 `envelope_assertion_failed` today, driven through
`record_only` which prints and asserts nothing:

| Command | Current reason (fragment) | Why left for `19-21` |
|---|---|---|
| `git --attr-source $T push --force origin main` | *"`$T` is the git verb for this command…"* | after the fix `$T` is an option's OPERAND, not a decision word |
| `git --attr-source *.x push --force origin main` | *"`*.x` is the git verb for this command…"* | same |
| `git --attr-source HEAD {push,--force} origin main` | *"a brace expansion splices words back into this command…"* | after the fix the braces sit after a consumed value, not in the verb slot |

Whether round 5's literalness bit still reaches those positions — and therefore
whether the identifier stays `envelope_assertion_failed` or becomes
`force_push_blocked` — is a `19-21` **design outcome this plan cannot know**.
**Asserting merely that they are REFUSED is also forbidden**: that would pin an
exit code whose mechanism this plan cannot name. `19-21` measures each and appends
the pin. This is exactly how `19-16` and `19-18` handled their undeliverable rows.

---

## The design question, answered — three rejected options and their costs

| # | Option | Verdict and cost |
|---|---|---|
| i | Complete the list | **REJECTED.** Closes today's two cells and is wrong again at the next git release; the list is *already* wrong in both directions. Completing an enumeration does not change the failure DIRECTION, which is the defect |
| ii | Derive the grammar from git at guard time | **REJECTED on two independent grounds.** (a) The guard is synchronous on `PreToolUse`; `push_needs_resolved_dests` exists because a reproduced 180–240 s hang made per-call shelling out unacceptable. (b) A guard that asks the program it guards to describe its own grammar can be lied to by a `git` earlier on `PATH` — the same surface this phase's own shims demonstrate. There is also no machine-readable enumeration: git's global options are prose, and `--list-cmds=` lists commands, not options |
| iii | Derive it at build or envelope-construction time | **REJECTED.** The probing binary is not the guarded binary; the result is non-hermetic; a probe that failed would have to fail closed, which is a guard nobody can build |
| iv | **ADOPTED** — invert the failure direction, pin the list against real git in a TEST | The knowledge is **one bit per option**. Make the ABSENCE of that bit a refusal, the same treatment `resolve_program`'s wrapper axis has. Structural rules that need no knowledge (attached `=`, `--`, a non-`-` token) run FIRST so the list has less to know. The better source of truth is the installed git binary, consulted at TEST time — it catches drift in both directions and already catches `--super-prefix` today |

---

## The measured over-refusal cost, from both sides

**On the installed git 2.43.0 the cost is ZERO.** Every option this git accepts is
classified by the probe and enumerated; the only rows moving permitted → refused
are ones git ITSELF rejects (`--bogus-opt`, `--super-prefix`, `-pc`, `-C/tmp`) —
refusals of commands that already do nothing.

**On a FUTURE git the cost is one refusal per newly added global option until the
constant learns it.** `--no-advice` and `--no-lazy-fetch` are the measured
stand-ins: real global options in later releases, rejected by this git, both in
the corpus so the future cost is checkable rather than argued.

**The user's recovery path, in the order the refusal message should offer it:**

1. **Spell the value ATTACHED (`--option=value`) where git accepts that form** —
   needs no list change at all, because git's own grammar makes an attached value
   self-contained. **Its limit is measured, not assumed:
   `git --shallow-file=/tmp/s version` → `unknown option:` on this git.** The
   safety claim is unaffected — an attached value can never consume a following
   word — but the step is not universally available, so it is offered first and
   **not offered alone**.
2. Drop the option.
3. Add it to the constant, which the drift pin will name.

The refusal names the **OPTION TOKEN**, on the same footing as the existing
`git -c {assignment}` refusals — actionable without quoting the command back
(SAFE-04).

**A correction it would be easy to generalise wrongly:** *"staleness costs
refusals rather than bypasses"* is true for the constants' **silence** and **false
for their entries**. A value-taking entry the runtime git rejects, or a
self-contained entry it treats as value-taking, SHIFTS THE VERB — the
`--super-prefix` direction. The fail-closed default covers absence; the contents
can still mis-index, which is why the drift pin must probe **both directions**.

---

## The third axis, and what makes it legible as one

`CALLEE_GRAMMAR_CLASSES` stands beside a **byte-identical** `UNREADABLE_CLASSES`
and a **byte-identical** `DELETION_CLASSES` — `git diff` shows **zero deletions
inside either region**, and no existing `MIN_*` constant, alphabet entry or
property was lowered, deleted or narrowed apart from the one named ratio
assertion.

Five classes with degenerate-proof predicates, each evaluated on a representative
spliced command:

1. consumes a **separate** word — `git --attr-source HEAD push`
2. consumes **no** word — `git --no-pager status`. **Class 1 cannot satisfy class
   2 and class 2 cannot satisfy class 1**, which is the whole grammar question
3. carries an **attached** value — needs no knowledge of git at all
4. an option the installed git **does not accept**
5. `--`, or a bare `-`

The predicates read `PROBED_VALUE_TAKING` / `PROBED_SELF_CONTAINED`, measured
two-sided against the real git binary — **which is what makes the classes
descriptions of the GRAMMAR rather than of the alphabet.** The
quoting-and-position control asserts that `git commit -m "--attr-source x"`,
`git push --attr-source HEAD --force origin main` and `git log --grep='--no-pager'`
satisfy **NONE**.

**Alphabets and floors, arithmetic stated and checked as exact equalities:**

- `GIT_GLOBAL_OPTIONS` — 15 class-tagged entries spliced **between the governed
  program and its decision words**, the only position `scan_leading` reads. Each
  entry's tag is asserted to agree with the predicate that actually fires and with
  its word count. `--attr-source HEAD` and `--shallow-file /tmp/s` are
  assert-by-name minimum entries and are the **only two** whose refused-arm cases
  are at exit 0 today — **16 of the 120**, 8 each.
- **A bare `-` is asserted mechanically ABSENT.** Every entry is spliced into
  refused bases and asserted refused, but `git - push --force origin main` is
  pinned PERMITTED before and after; a `-` entry would make the refused arm
  **permanently red in a file `19-21` may not edit** — `19-18`'s `{v}>` blocker one
  axis over. **The class-5 PREDICATE still recognises a bare `-`**, because
  narrowing it to match the alphabet would make the class a description of the
  alphabet rather than of the grammar.
- `GIT_GLOBAL_UNKNOWN_OPTIONS` — 4 entries, **asserted disjoint** and **never
  drawn by the invariance arm**, because an option the installed git rejects is
  refused after `19-21` even on a PERMITTED base and so is STRICTER than its base.
  It has its **own fail-closed property** over both base kinds.
- Counts: **210 cases / 120 refused / 90 permitted / 14 slots**; per-class
  **147 / 70 / 42 / 0 / 14**; plus **56** unknown-alphabet cases (32 on refused
  bases, already refused; **24 on permitted bases, all exit 0 today** — those 24
  are what makes the fail-closed property red). **The class-4 zero IS the
  disjointness assertion.**
- The generative property asserts the **exit code and an empty walk**, never a
  reason identifier — identifiers live in the named per-row pins where each
  carries its own written derivation — with the positive walk control preceding
  the loop.

---

## The anti-vacuity control, re-expressed — with the BYTE-measured ratio it replaces

Re-measured with the file's own stripper at `ec4c700`, **in BYTES, because Rust's
`len()` is a byte length and `policy.rs` carries multi-byte characters in its
prose**:

| File | raw bytes | stripped bytes | ratio | headroom |
|---|---|---|---|---|
| `src/envelope/policy.rs` | **263,360** | **65,947** | **25.0406%** | **107 stripped bytes ≈ 428 comment bytes** |
| `src/envelope/hooks.rs` | **99,909** | **33,460** | **33.4905%** | ample |

A `str`-CHARACTER measurement of `policy.rs` reads **262,748** — **612 bytes
light**, overstating the headroom by more than a factor of two.

**428 bytes of comment is less than `19-21`'s own doc additions.** The ratio had
stopped measuring what it names: its red would say *"the stripper broke"* while
meaning *"someone wrote comments"*, pulling directly against
`resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`, which
REQUIRES specific paragraphs to exist.

**Replaced, not supplemented, and not lowered:**

```
POLICY_MIN_PRODUCTION_BYTES = 40_000   // 61% of the 65,947 BYTES measured at ec4c700
HOOKS_MIN_PRODUCTION_BYTES  = 20_000   // 60% of the 33,460 BYTES measured at ec4c700
```

Keeping both would reproduce the collision the change exists to remove; a lower
percentage is the same control with a bigger budget — still red for comment
growth, still green under a proportional truncation. Ordinary refactoring that
deletes a third of either file still passes; a stripper that ate the logic drops
to near zero and turns red. The floor is deliberately **independent of comment
volume** and its failure message **forbids lowering it**. A grep for the ratio
expression over the file now answers **0**.

---

## The `glab --host` forge cell — recorded, callee UNCONFIRMED, out of scope

`glab --host gitlab.com mr create --title x` → exit 0 with **ZERO** ledger lines;
`glab --hostname …` → exit 0 with **one**. `FORGE_VALUE_OPTS` is
`["-R", "--repo", "--hostname"]` — the same hand-maintained enumeration of a
callee's option grammar, in a third component, failing in the **under-counting**
direction (SAFE-06).

**`glab` is NOT installed on this machine**, so whether glab accepts `--host` as a
separate-value global flag is **not confirmed against the callee**, and **this is
not claimed as a live bypass**. `gh` was swept and is clean — five spellings, one
ledger line each. Registered in `deferred-items.md` as a candidate for the round
after `19-21`, needing a machine with `glab` installed. **Deliberately not fixed;
`FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and `subcommand_word_indices` untouched.**

---

## `T-19-17r` — the bookkeeping gap, OUTSTANDING, and this plan did NOT accept it

`19-17-SUMMARY.md` calls it "accepted". Audits 5 and 6 both confirmed the
measurement and both pins (`tests/envelope_literal_decision.rs:1355-1379`) and
**both explicitly declined to make the acceptance, because accepting a risk is a
human decision.**

**There is still no Accepted-Risks-Log row and no register row.** The
Accepted-Risks-Log row count for that risk ID is **zero**, verified after this
plan's commits. **No `AR-19-13` row was added, and the word "accepted" is not
applied to `T-19-17r` in any test name, comment, this SUMMARY, or the appended
subsection.** The gap is recorded as OUTSTANDING; the next round either adds the
log row or drops the word.

---

## The `envelope_tracer` provenance correction

`19-19-SUMMARY.md` presents the `a_relocated_copy_of_the_stub_refuses_instead_of_acting`
`ExecutableFileBusy` failure as newly observed. `deferred-items.md` has carried it
**since 19-07** with the same symptom string, the same write-then-exec diagnosis
and the same "once in ~6 full-suite runs" frequency. Recorded as a **provenance
slip, not a defect**: audit 6 ran the test 8/8 green in isolation, and it fails
closed by ERRORING, so it cannot mask a regression. `19-19-SUMMARY.md` was **not
edited**; the correction is made once, in `19-SECURITY.md`'s plan-19-20 record.

---

## Deviations from Plan

**None affecting behaviour.** Two notes, both recorded rather than silent:

1. **The plan's section 3 lists nine exit-0 planning cells "assert exit 2", while
   a later, more specific sentence pins `git - push …` at exit 0 after the fix and
   section 7 claims five of the cells as composition rows.** Executed per the more
   specific instructions: section 3 asserts the three non-composition cells
   (`--super-prefix`, the after-a-known-option cell, the short bundle), the bare
   dash is a separate PERMITTED pin, and the five composition rows live in section
   7 where their point is made. No cell was dropped.
2. **The `POLICY_MIN_PRODUCTION_BYTES` doc originally quoted the replaced ratio
   expression verbatim**, which made a mechanical grep for it report the
   expression as still present. Rewritten to name the assertion in prose, so the
   grep answers 0 while the doc stays honest about what it replaced.

No auto-fixes under Rules 1–3 were required; no Rule 4 architectural decision
arose. No authentication gates occurred.

---

## Known Stubs

**None.** Every row in both files is either driven against the built binary and
asserted, or driven and printed through `record_only` with the reason it is not
asserted stated beside it.

---

## What remains uncovered

1. **`T-19-86` — OPEN at `high`, by explicit user scoping decision.** Its four
   rows still exit 0 and must: `git submodule foreach git push --force origin
   main`, `git rebase -x "git push --force origin main" HEAD~3`,
   `git bisect run sh -c "git push --force origin main"`,
   `git -c alias.p='!git push --force origin main' p`. Because it remains open at
   `high`, **this plan does not clear `/gsd-secure-phase 19`**.
2. **`T-19-91` — OPEN at `high`, arms unweakened, no remedy added.** `git reflog
   $S`, `git reflog show $S` and `git symbolic-ref $S` at exit 0 with **no second
   carrier**; bare `git push $REF` at exit 0 in-namespace. No decision-operand rule
   was added for `reflog`, `symbolic-ref` or `push`, and the denylist was not
   extended.
3. **`T-19-96`** — a glob in a PUSH FLAG, one slot outside the decision region.
   Registered by `19-16`, not fixed. Audit 6 notes it is one of four places a
   decision is made on a word the decision region does not cover.
4. **`T-19-74`** — core rows frozen and re-measured permitted.
5. **`T-19-84`, `T-19-85`, and `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision. Untouched.
6. **`T-19-100`, `T-19-101`, `T-19-102`** — open; the corpus exists and is RED,
   and `19-21` writes the rule.
7. **The `glab --host` forge cell** — registered, callee unconfirmed, out of scope.
8. **`T-19-17r`** — bookkeeping gap OUTSTANDING; the acceptance is deliberately
   unmade.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
`T-19-91` are both sub-classes of it and both remain OPEN at `high`, so **neither
this plan nor `19-21` clears `/gsd-secure-phase 19`.**

---

## Self-Check: PASSED

- `tests/envelope_callee_grammar.rs` — FOUND
- `tests/envelope_wrapper_class.rs` — FOUND (modified)
- `19-SECURITY.md`, `deferred-items.md` — FOUND (appended)
- Commits `a5404f7`, `fcdb9f7`, `f5f53bc` — all FOUND in `git log`
- `src/` hunks in each of the three: **0**
- `Cargo.toml` / `Cargo.lock` in the diff: **none**
- Accepted-Risks-Log rows for the `T-19-17r` risk ID: **0**
- Ratio expression `code.len() * 4` in `tests/envelope_wrapper_class.rs`: **0**
- `passed + failed` = **1631** = 1610 + 21 new `#[test]` fns
