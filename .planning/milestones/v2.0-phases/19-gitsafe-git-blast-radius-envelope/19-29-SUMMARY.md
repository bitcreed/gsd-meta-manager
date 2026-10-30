---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 29
subsystem: envelope
tags: [security, gap-closure, redirection-target, exact-path, credential-helper, ledger-bound, T-19-116, T-19-117, T-19-118, T-19-115]
requires:
  - "19-28 (the corpus, observed RED with zero `src/` hunks) — its RED list, PERMITTED list, SEGMENT-COUNT measurements, ledger curve and empty-helper result are this plan's handoff contract"
  - "audit 10 (19-SECURITY.md) — the ordered fix list and the honest limit"
provides:
  - "a pathname REDIRECTION-TARGET channel on `Segment::redirection_targets`, produced by the walk that already skips the target, with `segment.tokens` byte-for-byte unchanged"
  - "`policy::protected_carrier_named` — two word classes over two paths, the envelope directory as a PREFIX and the guard's own binary as an EXACT PATH"
  - "a REQUIRED empty-`credential.helper` pair in the injected `GIT_CONFIG_COUNT` triplet, verified by `git credential fill` failing closed"
  - "`ledger::MAX_LEDGER_BYTES` — a fail-CLOSED size bound derived from the deadline, not the caps"
  - "`SECTION_ENVELOPE`'s FIRST `Guaranteed` clause TRUE at 211 tokens of an unraised 215 cap"
affects:
  - "audit 11 — which judges whether T-19-116, T-19-117 and T-19-118 close; this plan says NARROWED"
tech-stack:
  added: []
  patterns:
    - "a fact about the COMMAND travels on the `Segment`, never as a token in the stream — the shape `Segment::redirection_unresolvable` already uses"
    - "two protected paths, two boundary KINDS: a PREFIX where the envelope owns the directory, an EXACT PATH where it does not"
    - "the criterion is the call that names the harm — `git credential fill`, never `git config --get-all`"
    - "a stale stated reason is corrected in place with why it was right when written, never edited away"
key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - src/envelope/ledger.rs
    - src/envelope/cred.rs
    - src/envelope/advisory.rs
    - src/envelope/mod.rs
    - tests/envelope_carrier_reach.rs
    - tests/envelope_control_carrier.rs
    - tests/envelope_credential.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The redirection target rides on the Segment, never as a token — proved falsifiable by the SEGMENT-COUNT pins rather than argued"
  - "The binary is an EXACT PATH and the envelope directory a PREFIX; the difference is measured, not aesthetic"
  - "current_exe() is resolved in guard_in rather than guard, because the corpus harness may not be edited and the claim is about the running process"
  - "The ledger bound is DEADLINE-derived, never cap-derived, because the caps are unclamped and agent-writable"
  - "T-19-115's tilde, glob and brace spellings get NO rule; the residue's arithmetic is corrected from four to seven instead"
metrics:
  duration: "one session"
  completed: 2026-09-04
actuals:
  tokens: 118000
  tasks: 4
  commits: 5
status: complete
---

# Phase 19 Plan 29: The rules — two paths, two word classes — Summary

## READ THIS FIRST

**`/gsd-secure-phase 19` is NOT cleared by this plan.**

- **`T-19-86` remains OPEN at `high`** by explicit user scoping decision, with
  `T-19-111` still moved OUT of it exactly as `19-27` moved it.
- **`T-19-91` remains OPEN at `high`.**
- **`T-19-111` remains OPEN at `high`, with NO rule written for it.**
- **`T-19-112` and `T-19-113` are NARROWED FURTHER and are still NOT CLOSED.**
- **`T-19-115` gets NO rule at all.**
- **Whether `T-19-116`, `T-19-117` and `T-19-118` close is audit 11's judgement
  rather than this plan's claim.** This plan says **NARROWED** and states what
  remains open.
- **Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

One-liner: the guard now establishes the path a line NAMES after a redirection
operator and the path of the binary it is running as — a `Segment`-borne channel
and an EXACT-PATH boundary raised at the one existing site — plus a required
empty-`credential.helper` injection that reads no command line at all, a
fail-closed ledger size bound derived from the deadline, and four honesty repairs
that are true against measurement.

## The five commits, in order

| # | SHA | What | `src/` |
|---|---|---|---|
| 1 | `a159f5e` | the AUTHORISED EXCEPTION — two rows whose PERMITTED verdict the rule changes | **0 hunks** |
| 2 | `20d06ac` | the redirection-target channel and the two-path predicate (`policy.rs`, +1106 −109) | yes |
| 3 | `48f9a74` | the binary threaded, the ONE call site widened (`hooks.rs`, +94 −19) | yes |
| 4 | `a1f1c92` | the honesty repairs, the credential control, the ledger bound | yes |
| 5 | `4e03c6d` | the records (`19-SECURITY.md` +611 −0, `deferred-items.md` +163 −0) | **0 hunks** |

**Five, not four.** The plan specified four; the authorised exception had to be
committed **first and on its own, before any rule commit**, which the user's
conditions required and which the plan's own four-commit shape did not carry.

## `19-28`'s RED set — confirmed still RED before any production line moved

Re-run against the unmodified tree at `d5542c6` with `--no-fail-fast`, verbatim:

```text
passed=1802 failed=6 ignored=13 total=1808 result_lines=46
envelope binaries: 17

    a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29
    after_19_29_a_command_whose_operand_is_the_guards_own_binary_is_refused
    after_19_29_a_ledger_past_the_size_bound_refuses_while_one_just_under_it_still_counts
    after_19_29_a_redirection_that_writes_the_generated_gitconfig_is_refused
    after_19_29_ordering_pin_a_holds_for_a_redirection_target_too
    after_19_29_ordering_pin_b_holds_for_the_binary_carrier_too
```

**Identical to `19-28-SUMMARY.md`'s recorded contract in every number and every
name. No row was already green**, so no rule here is a rule nobody showed was
needed. None of the three documented flakes fired in the baseline run.

### Their transition

| row | before | after | landed in |
|---|---|---|---|
| `after_19_29_a_redirection_that_writes_the_generated_gitconfig_is_refused` | RED | **GREEN** at `envelope_assertion_failed` | commit 2 |
| `after_19_29_ordering_pin_a_holds_for_a_redirection_target_too` | RED | **GREEN** at `envelope_assertion_failed` | commit 2 |
| `after_19_29_a_command_whose_operand_is_the_guards_own_binary_is_refused` | RED | **GREEN** at `envelope_assertion_failed` | commit 3 |
| `after_19_29_ordering_pin_b_holds_for_the_binary_carrier_too` | RED | **GREEN** at `envelope_assertion_failed` | commit 3 |
| `a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29` | RED | **GREEN** | commit 3 |
| `after_19_29_a_ledger_past_the_size_bound_refuses_while_one_just_under_it_still_counts` | RED | **GREEN** at `envelope_assertion_failed` | commit 4 |

## `19-28`'s PERMITTED set — re-checked, and THREE rows were not still permitted

The seven spelling rows in both word positions, the expansion-borne, symlinked
and relative rows, the near-misses and **both EXACT-PATH-not-PREFIX binary
controls** are all still PERMITTED. `envelope_control_carrier` is 37/0 and
`envelope_wrapper_class` is 52/0.

**Three rows flipped. Every one is a row whose own stated derivation says the fix
flips it, and each was verified by measurement before it was touched.**

## WHAT THE AUTHORISED EXCEPTION MEASURED

Driven against the **BUILT BINARY**, fresh `GSD_MM_ENVELOPE_ROOT` per row,
envelope directory walked after each, **before any assertion was edited**:

```text
exit=2  reason=envelope_assertion_failed  : > <ENV>/alpha/pr-ledger.ndjson
exit=2  reason=envelope_assertion_failed  printf 'exit 0' > <ENV>/alpha/hooks/pre-push
exit=2  reason=envelope_assertion_failed  echo evil > <ENV>/alpha/askpass
exit=0  reason=(permit)                   : > /tmp/l                        <- CONTROL
exit=0  reason=(permit)                   D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson
exit=0  reason=(permit)                   rm -f /tmp/l
exit=0  reason=(permit)                   rm -f pr-ledger.ndjson
```

**All three rows flip, each flip is the intended one at the intended identifier,
and NO FOURTH ROW in that file flips** — `envelope_control_carrier` measured
36 passed / 1 failed with the rule applied: one test, three rows. Directions
(ii), (iii) and (iv) are untouched.

**Both stated reasons were rewritten, not deleted:**

- *"These rows will not flip"* is **FALSE by measurement.** `T-19-118` is a live
  `high` credential reach through direction (i) and it re-opens the closed
  `T-19-23`. The inference went from a mechanism that is real to a conclusion the
  mechanism does not support: *the rule cannot see it as a WORD* does not entail
  *the guard cannot establish it at all*.
- *"A rule that read redirection targets would re-open a model five rounds have
  pinned shut"* is **false in a more interesting way** — it assumed the only way
  to read a target is to make `>` a separator or push a token into the stream.
  The `Segment`-borne fix does neither, and the pins prove it: `SEPARATORS`
  byte-identical, `is_separator(">")` still `false`, `segment.tokens`
  byte-identical, round 6's over-deletion control still permitting.

Both were **sound when written and were overtaken by evidence**, and both say so
in place. This phase has three disclosures that described a control's coverage in
terms which quietly excluded the live case; this round did not add a fourth.

## The two rows the authorisation did NOT name — reported as findings

1. **`the_ledger_inflation_spellings_are_all_permitted_today`**
   (`tests/envelope_carrier_reach.rs`). **`19-28`'s blocking scope finding
   checked for exactly this shape and scoped the check one file too narrowly** —
   it reported *"no OTHER test file names an envelope-root redirection target as
   a pinned-permitted row"*, true of every file except the one it was written in.
   The row's own comment said *"they are the ones `19-29`'s widened SIGHT turns
   to exit 2"* while its assertion was written at the pre-fix verdict —
   `19-28`'s own deviations 2 and 3, missed once. Measured before correction:
   both spellings exit 2 at `envelope_assertion_failed`, both outside-the-root
   controls exit 0.

2. **`t_19_117_the_ledgers_size_is_unbounded_work_on_the_guards_critical_path`**
   (`tests/envelope_carrier_reach.rs`). **This row and the fix are MUTUALLY
   EXCLUSIVE BY CONSTRUCTION.** It asserts in the present tense that a
   202,000,000-byte ledger crosses the deadline, while
   `after_19_29_a_ledger_past_the_size_bound_…` in the SAME file requires the
   bound to sit inside `(1 MiB, 24 MiB)`. **No constant satisfies both.**
   `19-28`'s own failure text anticipated it: *"If this does not reproduce,
   RECORD the measured curve and report it rather than asserting audit 10's
   number."*

Both were corrected under the same three conditions the authorisation set —
measured first, reasoning rewritten in place, nothing made uniform — and are
reported here rather than absorbed into the authorisation.

## Tasks completed

**All four. `T-19-117` was NOT severed** — this plan's one named severable item
was executed in full.

## The direction-(i) answer, with its three source-read facts

**Closing direction (i) does NOT require the deletion model to read redirection
targets as argv.** Round 6 answers *which words ARRIVE*; a redirection target
does not arrive, and that answer did not change. Rule (a) asks *does this line
NAME a path this run's controls live in?*

1. **`skip_redirection_target` already walked the target's full extent** with the
   target's own quoting rules. Its text and literalness are BY-PRODUCTS. Its
   doc's *"Its text is never needed, only its extent"* is corrected in place with
   the reason it was right when written (WR-02).
2. **`redirection_operator_len` was a CLOSED twelve-operator grammar**, and only
   SEVEN take a PATHNAME (`<`, `>`, `>>`, `>|`, `<>`, `&>`, `&>>`); five take a
   heredoc DELIMITER, a here-STRING or an fd NUMBER. **The split is derived AT
   THE OPERATOR**, in the match that already computes the length. Renamed
   `redirection_operator`, because a name saying "len" returning pathname-ness
   would be the same stale-reason defect one function over.
3. **`split_segments_with_heads` excludes operator tokens and FLUSHES at one.** A
   target token would split `git >/dev/null push --force origin main` into
   `[git]` + `[push, --force, origin, main]`, and the bare `git` resolves
   `Governed` with an EMPTY argv, which `classify_git` answers `Allow` for.

### The SEGMENT-COUNT measurements, before and after

```text
                                          BEFORE                        AFTER
git >/dev/null push --force origin main   1 segment, [git,push,         UNCHANGED
                                           --force,origin,main]
git x2>/tmp/o push --force origin main    1 segment, [git,x2,push,      UNCHANGED
                                           --force,origin,main]
git 2>/dev/null push --force origin main  1 segment                     UNCHANGED
git <<EOF push --force origin main        1 segment                     UNCHANGED
git >                                     1 segment, unresolvable       UNCHANGED
is_separator(">") / ("<") / ("&&")        false / false / true          UNCHANGED
SEPARATORS at policy.rs:2297              byte-identical (one commit ever, 84a9b05)
```

**THE COST:** ONE additive `Segment` field, a private return-type change on
`tokenize` with FOUR call sites, a corrected stale doc sentence, and a
pathname/non-pathname split of a grammar the file already enumerated. **The plan
projected TWO additive fields; ONE was enough** — `tokenize`'s return type
carries the attribution index, so no `Token` field was needed. **No second
reading site and no second scan was needed.**

## The rule's conditions, over both word classes and both paths

**Two word classes:** the segment's own literal words, and its pathname
redirection targets. **Two paths, two boundary KINDS:**

- **the envelope directory, as a PREFIX** — this envelope owns every byte under
  it, and `rm -rf <root>/<alias>` takes nine carriers in one call;
- **the guard's own binary, as an EXACT PATH** — its directory is shared with
  everything else the user installed, so a prefix would refuse `ls <parent>` and
  every `cargo install`. `cp /bin/true <BINARY-PARENT>/some-other-file` and
  `ls <BINARY-PARENT>` are pinned PERMITTED and are still exit 0.

**An absent binary path makes that half SILENT** — fail-open, stated on the
signature, and pinned with a positive control.

**Five placement consequences** are stated at the one call site: the original
three, plus that the targets come from a field the ONE walk filled with
`entry.tokens` unchanged, and that the two paths carry two different boundary
kinds. The ordering pins land at deliberately different identifiers —
`envelope_assertion_failed` for the carrier rows, `force_push_blocked` for all
four outside-the-protected-set controls.

## The SEVEN fail-open directions, in the docs' own words

**Stated as a CONDITION, not a list.** The rule is silent about **a word the
SHELL MAY REWRITE**, about **a word that IS NOT ABSOLUTE**, and about **a word
that reaches a protected path ONLY THROUGH A LINK** — in EITHER word class and
over BOTH paths. Seven spellings are INSTANCES: (i) a redirection target that is
any of the other six, (ii) expansion-borne, (iii) symlinked, (iv) relative, (v)
tilde, (vi) glob, (vii) brace list.

- **(i) is NARROWED to those seven and is NOT closed** — it becomes a second WORD
  CLASS the other six apply over.
- **(iii) and (iv) keep their measured partial mitigations over the DIRECTORY and
  have NONE over the binary.**
- **(v), (vi) and (vii) get no rule and none is possible**: a tilde needs the
  environment, a glob needs the filesystem, both forbidden at guard time.

**The path set grew, the word set grew, and the silences did NOT shrink. NOT ONE
has an automated control.** **No revisit condition, no pin, no schedule and no
version witness was created for any of them** — `19-25`'s witness schedules a
re-audit against a FUTURE GIT, and all seven are reachable by the agent TODAY, so
a schedule would observe a version string that never moves while the residue is
exercised. They appear in the predicate's doc, the call site's comment,
`cred.rs`'s WHAT IS NOT COVERED and both records.

## `C-10`'s partial rule, with what it does not reach at the same weight

**Refused:** `cp`, `mv`, `install -m 0755`, `ln -f` and `printf 'x' >` over the
binary's ABSOLUTE LITERAL path.

**NOT reached:** an expansion-borne spelling, a tilde spelling, a relative
spelling after a `cd`, and **a link on `PATH` whose target `current_exe()`
reports instead.** All pinned PERMITTED and still exit 0.

**`current_exe()`'s measured agreement:** `/proc/self/exe` is fully resolved and
`hooks::install` bakes the same call's answer, **so the baked path and the
guard's path agree by construction**; the residue is the symlink case above.
**Silent-when-absent is recorded as fail-open. `T-19-01` is unweakened and was
never a defence of the exec target's bytes.** `C-10`'s letter is now
**(a), EXACT PATH, PARTIAL**.

**A new row pins that the clause protects the RUNNING binary and not every copy
of it**: a byte-identical copy of the product binary at another path stays
PERMITTED, beside a positive control at the running binary — which forbids a
basename rule behaviourally as the no-program-names fence forbids one
mechanically.

## The `SECTION_ENVELOPE` repair

**COMPLETE PIN HIT SET, enumerated with `grep -rn` BEFORE the text moved:**

| site | what it pins | outcome |
|---|---|---|
| `tests/envelope_advisory.rs:182-221` | all three phrases, in order | **passed UNMODIFIED** |
| `tests/envelope_advisory.rs:239-289` | the 215-token cap, the 80-column cap | **passed UNMODIFIED, cap NOT raised** |
| `tests/envelope_advisory.rs:302` | `envelope_notice` renders it | **passed UNMODIFIED** |
| `tests/envelope_control_carrier.rs:2069` | the CAP clause verbatim (`19-27`'s) | **untouched, passed UNMODIFIED** |
| `tests/envelope_control_carrier.rs:2090-2092` | the phrase-ORDER assertion | **passed UNMODIFIED** |
| `src/driver/dry_run.rs:549, 735, 796` | three consumers | **passed UNMODIFIED** |
| `src/envelope/advisory.rs:328` | `envelope_notice`'s composition | **untouched** |

**No existing assertion pins the credential clause literally**, so the plan's
reserved "one pre-authorised edit" **was not needed and was not taken**; the
repair is covered by a NEW assertion instead.

```text
BEFORE  This run cannot reach your ambient git credentials or SSH agent: the
        socket is removed, not emptied, and global and system git config is a
        generated file naming no credential helper.

AFTER   As started, this run cannot reach your ambient git credentials or
        SSH agent: the socket is removed and the git config it is given
        runs no credential helper.
```

*"As started"* states what the envelope ESTABLISHES and stops. **It makes no
completeness claim and enumerates nothing.** `runs no credential helper` is
measured, not stylistic: `--get-all` still LISTS the helper while the fill fails
closed, so *"names no helper"* would now be false where *"runs"* is true.

**THE TOKEN ARITHMETIC:**

```text
before          213 tokens of a 215 cap   (two of headroom)
Guaranteed      31 -> 28   dropping a CLAIM ("not emptied"; both scopes named)
Not guaranteed  23 -> 24   GENERALISING a LIMITATION, not dropping one:
                           "the hook stubs and ledger the envelope installed"
                        -> "the files and the binary this envelope runs on"
after           211 tokens.   **THE CAP WAS NOT RAISED.**
```

The generalisation is one token longer and covers `T-19-116`, a route the old
wording did not. `cannot reach your ambient git credentials` survives **verbatim,
first and unwrapped** on one 69-character line; widest line 74 of 80; `\x20`
indentation checked on the **RENDERED** constant; **no residual disclosure
dropped.**

## The REQUIRED mechanism control

**One `credential.helper` pair with an EMPTY value, through the existing
`config_env` builder** — never a second construction site. Measured against real
git 2.43.0 **before the assertion was written**:

```text
                            --get-all credential.helper      credential fill
no injected pair            `store`             exit 0       ambient secret PRESENT
EMPTY-helper pair injected  `store`, then an    exit 0       secret ABSENT
                            EMPTY line
+ later -c credential.helper=…                               secret PRESENT
```

**The query still LISTS the helper while the fill fails closed** — git's empty
value resets the helper list that **RUNS**, not the list the query **ENUMERATES**.
**A gate built on `--get-all` reports a working control as broken.** The criterion
is the call that names the harm, and `--get-all`'s reading is RECORDED beside it
in the test and in the doc.

**The asserting row drives a TILDE write** — one of the three spellings rule (a)
does NOT reach and never will — so the row proves the control is **wider than the
rule** rather than duplicating it. The row reads the pairs from
`cred::hooks_path_env` itself rather than re-spelling them, and drives its own
harm control first.

**Cost:** `GIT_ASKPASS` UNTOUCHED, `gh` unaffected, and the later
`-c credential.helper=store` override recorded as the **one BOUNDED residue —
bounded because that spelling is ARGV-VISIBLE and already governed**, unlike
every write spelling.

**`AR-19-04` is NOT un-accepted** — its reasoning gap (tampering BETWEEN runs vs
credential REACHABILITY DURING one) is recorded beside it and nothing more.
**`T-19-23` is NOT marked closed.**

## The ledger bound — NOT severed

**`MAX_LEDGER_BYTES = 8 MiB`, DEADLINE-derived and NOT cap-derived.** `tally`
costs ≈ 41 ms/MB; 8 MiB ≈ 340 ms, **under 7% of `GUARD_TIMEOUT_SECS = 5`**. A
cap-derived bound would be a bound the agent steers — the caps are unclamped
(`C-15`) and read from an agent-writable file, **and they were not clamped.**

**It REFUSES rather than reading less**, against `tally`'s no-under-count
invariant: a tail read, a line cap and sampling all UNDER-COUNT, violating it in
the direction that permits. The contrast is quoted from `ends_mid_line` three
functions above. The check is a **`stat` before the read AND before the append**,
so the refusal is fast — a bound that refused only after reading would answer the
harm with the harm. `Err` rather than `CapVerdict::Refuse`, so the guard parks at
**`EnvelopeAssertionFailed`, never `PrCapExceeded`** (D-24), naming the file and
saying it can be archived (AR-19-11).

```text
   ledger bytes    gh pr create        git push --force    ls / git status
   0                       0 ms exit 0            0 ms exit 2        0 ms
   940,000                41 ms exit 0            0 ms exit 2        0 ms   <- under
   18,800,000             10 ms exit 2            0 ms exit 2        0 ms   <- past
   188,000,000             2 ms exit 2            0 ms exit 2        0 ms   <- past
   BASE (19-28, no bound): 7900 ms at the last level, PAST the 5 s deadline.
```

**Over-refusal, from both sides:** it refuses the forge commands the cap governs
**and nothing else** — every `git` command and every ordinary shell call at the
same level still reaches its own verdict. The 1 MiB control **still permits AND
still counts**. **The BEHAVIOURAL half is UNMEASURED and claimed in NEITHER
direction.**

## `mod.rs:165-168` — before and after

```text
BEFORE  "…That party can equally unset `GIT_CONFIG_COUNT`, which is D-09's stated
         ceiling, so this override does not lower a boundary that was standing."

AFTER   the conclusion is KEPT and re-grounded on the variable's measured
        INERTNESS, and the paragraph now names the FILE route and the BINARY
        route as ceilings the environment route does not bound — with
        `env -u GIT_CONFIG_COUNT git push --force` recorded as REFUSED beside them.
```

**`19-27` read this paragraph, reached the same conclusion and correctly DECLINED
to edit it** because `mod.rs` was outside its stated file set — its own stated
fallback rather than a silent widening. Audit 10 endorsed the decline. **ZERO
non-doc lines changed**, verified by the filtered `git diff --unified=0` returning
nothing.

## The SIX NAMED FILE EXCEPTIONS, stated AS exceptions

| file | bound | verification |
|---|---|---|
| `policy.rs` | the channel, the `Segment` field, the predicate and its doc, the refusal, `842`→`421`, additions in the existing `mod tests` | `resolve_program`, `resolve_program_with_head`, `first_unreadable_decision_word`, `config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api`, `scan_leading`, `forbidden_repo_path`, `SEPARATORS`, `is_separator` — **ZERO** diff lines; ONE `#[cfg(test)]` |
| `hooks.rs` | `guard_in`'s doc/preamble, `classify_segments`' signature and recursion, the ONE call site and its comment | every other named item **ZERO**; the `settings_json` second-carrier row **NOT repaired** |
| `ledger.rs` | `record_and_check_in`'s guard and `MAX_LEDGER_BYTES` — **first plan in the phase to open this file** | `tally`, `append_entry`, `ends_mid_line`, `parse_stamp`, `saturating_bump`, `ledger_path`, `ledger_path_in`, `LedgerEntry`, `Tally`, `CapVerdict`, `WINDOW_SECS` — **ZERO** |
| `cred.rs` | the reach/limit paragraphs, `write_gitconfig`'s doc, the ONE injected pair | `write_gitconfig_in`, `gitconfig_body`, `write_askpass_stub_in`, `configured_remote_host`, `EnvelopeEnv`, `build_env_in` untouched |
| `advisory.rs` | `SECTION_ENVELOPE`'s literal and its doc | `envelope_notice`, `protection_line`, `PROTECTION_WARNING`, `PROTECTED_CLAIM` — **ZERO** |
| `mod.rs` | `ENVELOPE_ROOT_ENV`'s residual-limit paragraph | **ZERO non-doc lines** |

**`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` not taken on; `scan.rs` and
`config.rs` not opened; `Cargo.toml`/`Cargo.lock` untouched (`T-19-SC`).**

**ASSERTIONS UPDATED, by test function name — five, each a named exception:**

| test function | file | why |
|---|---|---|
| `direction_i_…_stays_permitted` → `after_19_29_direction_i_a_redirection_target_is_read_and_the_control_outside_the_root_is_not` | `envelope_control_carrier.rs` | the human-authorised exception |
| `the_ledger_inflation_spellings_are_all_permitted_today` → `after_19_29_the_ledger_inflation_spellings_are_reached_and_the_outside_controls_are_not` | `envelope_carrier_reach.rs` | its own comment already stated the flip |
| `t_19_117_the_ledgers_size_is_unbounded_work_…` → `after_19_29_t_19_117s_unbounded_work_is_bounded_and_the_forge_split_is_unchanged` | `envelope_carrier_reach.rs` | mutually exclusive with the fix it demands |
| `a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope` | `envelope_credential.rs` | criterion moved from the QUERY to the EFFECTIVE VALUE |
| `the_triplet_names_the_count_the_key_and_the_directory` → `the_injected_pairs_name_the_count_the_keys_and_their_values` | `cred.rs`'s own `mod tests` | the injection carries a second pair; the pin is WIDENED |

## The `421` correction

`policy.rs:7450` read *"842 multi-byte characters"*. **842 is the
byte-minus-character DIFFERENCE; the count is 421** — each is a THREE-byte
character contributing TWO extra bytes, and 421 × 2 = 842. **Every other number
in that comment was re-derived by audit 10 and none was touched**; the floor's
value and strictness do not move.

## Mechanism pins and byte floors, re-measured

`SEPARATORS` **byte-identical** at `policy.rs:2297`; `is_separator(">")` `false`,
both asserted by a new pin with a positive control. Round 5's literalness bit is
non-vacuous and **now read in a THIRD place**. Round 6's deletion model unchanged
in behaviour, its over-deletion control still permitting. Rounds 7, 8, 9 and 10's
clauses all green. **Not one config-resolution verdict moved**
(`envelope_config_resolution` 30/0). Byte floors, the deep anchor, the sentinel
counts and the stripper's three protections unchanged; ONE `#[cfg(test)]` line in
`policy.rs`, ONE in `hooks.rs`.

## The gate

```text
command   rtk proxy cargo test --no-fail-fast   (counts read with `rtk proxy grep`, D-34)
before    1808  passed + failed   (1802 passed, 6 failed, 13 ignored, 46 result lines)
after     1820  passed + failed   (1820 passed, 0 failed, 13 ignored, 46 result lines)
delta     +12
check     12 new `#[test]` fns across all five commits, 0 removed
          (git diff HEAD~5..HEAD -- src/ tests/ | grep -c '^+\s*#\[test\]' = 12, '^-' = 0)
          1808 + 12 = 1820.   **The identity holds exactly.**
```

**A red test RAN, so red→green leaves the total unchanged and every increase
comes ONLY from new `#[test]` fns. THE RED SET FROM `19-28` IS EMPTY.**

**SEVENTEEN `envelope_*` binaries RAN**, all 0 failed:

| binary | passed | failed |
|---|---|---|
| `envelope_advisory` | 10 | 0 |
| `envelope_argv_deletion` | 20 | 0 |
| `envelope_callee_grammar` | 19 | 0 |
| **`envelope_carrier_reach`** | **39** | **0** |
| `envelope_command_position` | 18 | 0 |
| `envelope_config_resolution` | 30 | 0 |
| `envelope_control_carrier` | 37 | 0 |
| `envelope_credential` | 6 | 0 |
| `envelope_expansion_slots` | 32 | 0 |
| `envelope_hook_refusals` | 7 | 0 |
| `envelope_literal_decision` | 43 | 0 |
| `envelope_pr_cap` | 11 | 0 |
| `envelope_reparsed_value` | 34 | 0 |
| `envelope_tracer` | 6 | 0 |
| `envelope_wiring` | 14 | 0 |
| `envelope_wrapper_bypass` | 13 | 0 |
| **`envelope_wrapper_class`** | **52** | **0** |

`cargo build` and `cargo clippy -- -D warnings` exit 0. **`cargo clippy --tests`
is NOT the gate** — it already fails at base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs`, untouched.

### Documented flakes — TWO FIRED and are RECORDED rather than reported clean

The **preceding** full run (`/tmp/19-29-t3.log`, after commit 4) failed on both
`tests/driver_reattach.rs` rows:

```text
    a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step
    a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired
```

**Re-run in isolation: `driver_reattach` 3 passed / 0 failed** — the documented
signature. They were not fixed and are out of scope. The **final** gate run is
fully green, which is not evidence they are fixed.

**The `envelope_tracer` ETXTBSY race did NOT fire** in any run of this plan;
`envelope_tracer` is 6/0. **That race is `C-10`'s own seam and this round changed
what the guard does about that seam**, so its absence is RECORDED with its
relation to this round's change stated in **NEITHER direction**: absence is not
evidence it is fixed, and not evidence this round touched it.

## Deviations from plan

### 1. [Rule 3 — blocking] `current_exe()` is resolved in `guard_in`, not in `guard`

**Found during:** Task 2 design, before any line moved.
**Issue:** the plan required `guard` to resolve `current_exe()` and hand it to
`guard_in`, following `install_in`'s precedent. **That is unsatisfiable without
editing eleven fenced evidence files**: `guard_in` is the explicit-roots entry
point **twelve** test files drive directly through a fixed 7-argument harness,
including `tests/envelope_carrier_reach.rs`'s `ask_full`, in a file this plan may
only ADD to. It is also wrong on the merits: `install_in` takes `binary`
explicitly because it **bakes** the path into a stub, and a stub carrying the
test binary would be wrong; this clause is the opposite case — its claim is about
*the binary the process answering the guard call is running as*, which
`this_binary()`'s own doc in the corpus states. Resolved in `guard`, the path
would be absent for every corpus row and the clause would be **silently
untested**.
**Fix:** resolved once at the top of `guard_in`, above the segment walk, threaded
into `classify_segments` and through the `NestedPayload` recursion. Once per
invocation either way — `guard` calls `guard_in` exactly once — and nothing below
it makes a process or filesystem call, asserted by the no-filesystem source pins.
The reasoning is recorded on `guard_in`'s doc.
**Files:** `src/envelope/hooks.rs`. **Commit:** `48f9a74`.

### 2. [Rule 3 — blocking] Task 1's commit adapts the one `hooks.rs` call site

**Found during:** Task 1, at the build.
**Issue:** the plan required `hooks.rs` to show ZERO diff lines in Task 1's
commit. `protected_carrier_named` has exactly ONE caller and **the tree must
build at every commit**.
**Fix:** the one call site is adapted mechanically in commit 2 — same placement,
same channel, same identifier, `binary` still `None`. A one-commit compatibility
shim was the alternative and was declined: it would have put a deletion of
never-differently-behaving code into commit 3, making **Task 2's diff LESS
reviewable**, which is the property the split exists to protect.
**Files:** `src/envelope/hooks.rs`. **Commit:** `20d06ac`.

### 3. [Rule 1 — the plan's own criterion, in reverse] `envelope_credential.rs`'s criterion moved

**Found during:** Task 3, on the first run after the pair landed.
**Issue:** `a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope`
asserted that `git config --get credential.helper` **fails**. With the empty pair
the query **succeeds** and prints an empty line — **the plan's own documented
false negative, in reverse: a gate on the query's exit status reports a control
that got STRONGER as one that broke.**
**Fix:** the criterion is the EFFECTIVE VALUE — what git would run must be empty
and must not be the user's `store` helper — with the file's own control
unchanged. That file is not in this plan's fenced list.
**Files:** `tests/envelope_credential.rs`. **Commit:** `a1f1c92`.

### 4. [Rule 1 — my own process defect] The step-1 enumeration was truncated

**Found during:** Task 3.
**Issue:** my first `grep -rn` over `tests/` for `credential.helper` was piped
through `head -40` and **missed `tests/envelope_credential.rs` entirely** —
**D-34's own lesson, in this plan's own execution.** The row above was found by
running the suite, not by the enumeration that was supposed to find it.
**Fix:** the enumeration was re-run untruncated and the complete hit set is in the
`19-SECURITY.md` record.
**Commit:** `a1f1c92`.

### 5. [Rule 2] Five commits rather than four

The authorised exception had to be committed **first and on its own, before any
rule commit**, which the user's conditions required and the plan's four-commit
shape did not carry.

### 6. Design note, not a deviation: ONE additive field, not two

The plan projected two additive fields (`Token` and `Segment`). One was enough —
`tokenize`'s return type carries the attribution index — so no `Token` field was
added. Fewer fields is strictly less surface; recorded rather than glossed.

## Found and NOT fixed

1. **`19-28`'s blocking scope finding was one file too narrow.** It reported *"no
   OTHER test file names an envelope-root redirection target as a pinned-permitted
   row"* — true of every file except the one it was written in. Two further rows
   in `tests/envelope_carrier_reach.rs` flipped. Corrected; the underlying
   process lesson is recorded, not fixed.
2. **`tests/envelope_carrier_reach.rs`'s self-assertion at `:2846-2861` is now
   partially stale.** It forbids any line containing both `GIT_CONFIG_KEY_1` and
   `assert`, to stop `19-28` asserting the candidate control — while its own text
   says *"`19-29` REQUIRES it and `19-29` asserts it."* This round's assertion
   reads the pairs from `cred::hooks_path_env` rather than re-spelling the key,
   so the fence stays GREEN and non-vacuous for what it guards. **Left as is**:
   it is not failing, and editing a green evidence row for tidiness is not
   something this plan may do.
3. **The `=`-joined global-option spelling** (`git --git-dir=<ENV>/alpha push …`
   answering `force_push_blocked` rather than `envelope_assertion_failed`) is
   carried forward from `19-28` unchanged. Not a defect; recorded because the
   space-separated twin answers differently.
4. **`C-08`'s behavioural half** stays UNMEASURED, claimed in neither direction;
   the `settings_json` second-carrier row was NOT repaired.
5. **`T-19-117`'s behavioural half** — what the agent CLI does past its
   registered timeout — stays UNMEASURED, a property of a closed-source binary.
6. **`glab` is not installed**, so its forge cell stays unmeasurable against its
   callee. No pin that would skip was written, and `FORGE_VALUE_OPTS` keeps
   `--hostname`.

## Known Stubs

None. No stub, placeholder or hardcoded empty value was introduced. The one
empty value in this round — `GIT_CONFIG_VALUE_1=""` — **is the mechanism**: an
empty `credential.helper` resets git's helper list, and a non-empty value there
would be the opposite of the control. Its doc says so at the constant and in the
test.

## What remains uncovered

1. **`T-19-86`** — **OPEN at `high` by explicit user scoping decision.** Four
   registered rows at exit 0, pins green and UNMODIFIED, `T-19-111` kept OUT of
   it.
2. **`T-19-91`** — OPEN at `high`, three arms unweakened.
3. **`T-19-111`** — OPEN at `high`, **no rule**, corpus rows RECORDED in neither
   direction; `19-27`'s five-site attribution correction stays as performed.
4. **`T-19-112`, `T-19-113`** — NARROWED FURTHER, still NOT CLOSED; `C-15` and
   the deferred ledger option (b) are routes these rules do not reach.
5. **`T-19-115`** — OPEN at `medium`, **NO RULE**, no acceptance made.
6. **`T-19-116`, `T-19-117`, `T-19-118`** — **NARROWED, not closed.** Residues
   stated above; audit 11 judges whether any closes.
7. **`T-19-96`, `T-19-110`, `T-19-74`** — registered open; `T-19-74`'s core rows
   frozen.
8. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted;
   `scan.rs` and `config.rs` not opened.
9. **`C-08`'s behavioural half** and **`T-19-117`'s behavioural half** —
   UNMEASURED, claimed in neither direction.
10. **`C-15`'s `pr_cap_*`** — unclamped, deliberately: clamping a configured cap
    is a PRODUCT decision, not a guard rule.
11. **`T-19-17r`** — **OUTSTANDING for the TWELFTH time.** This plan did NOT
    accept it, added no Accepted-Risks-Log row, created no `AR-19-13`
    (`grep -cE '^\| AR-19-13 \|'` is **0**), and applied the word "accepted" to it
    nowhere. `AR-19-04` and `AR-19-05` are carried forward not un-accepted, not
    re-rated and not renumbered.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

## Self-Check: PASSED

- `src/envelope/policy.rs` — FOUND
- `src/envelope/hooks.rs` — FOUND
- `src/envelope/ledger.rs` — FOUND
- `src/envelope/cred.rs` — FOUND
- `src/envelope/advisory.rs` — FOUND
- `src/envelope/mod.rs` — FOUND
- `tests/envelope_carrier_reach.rs` — FOUND
- `tests/envelope_control_carrier.rs` — FOUND
- `tests/envelope_credential.rs` — FOUND
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md` — FOUND
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` — FOUND
- commit `a159f5e` — FOUND
- commit `20d06ac` — FOUND
- commit `48f9a74` — FOUND
- commit `a1f1c92` — FOUND
- commit `4e03c6d` — FOUND
- `grep -cE '^\| AR-19-13 \|' 19-SECURITY.md` — 0
- `19-SECURITY.md` diff — 611 insertions, **0 deletions** (a pure append)
- `SEPARATORS` at `policy.rs:2297` — byte-identical
- full suite — 1820 passed / 0 failed / 13 ignored, 17 `envelope_*` binaries
