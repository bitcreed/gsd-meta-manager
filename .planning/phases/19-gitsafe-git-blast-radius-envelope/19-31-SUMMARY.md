---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 31
subsystem: envelope
tags: [security, gap-closure, interior-path, attachment-boundary, credential-helper, ledger-kind, wr-02, T-19-119, T-19-120, T-19-121, T-19-116]
requires:
  - "19-30 (the round-12 corpus) — its recorded RED and PERMITTED lists are this plan's handoff contract"
  - "audit 11 (19-SECURITY.md) — the closure order (a) T-19-119, (b) T-19-121, (c) T-19-120"
provides:
  - "the interior-path rule: every `/`-anchored substring of a LITERAL word, at the ONE existing reading site"
  - "the residue restated as a CONDITION over what the predicate READS, with NO count"
  - "the `credential.helper` by-name clause in scan_leading's leading-option region"
  - "the ledger's KIND bounded inside the existing `stat`, with the `Err` arm untouched"
  - "both false claims corrected under the WR-02 discipline — policy.rs:5743 and cred.rs:420-425"
  - "a green `--no-fail-fast` gate: 1871 = 1859 + 12, ZERO failures, 18 envelope_* binaries"
affects:
  - "audit 12 — which judges whether anything closes; this plan claims nothing closed"
tech-stack:
  added: []
  patterns:
    - "a boundary over what the predicate READS, never over a list of attachment characters"
    - "containment of the NORMALISED STRING rather than of the index"
    - "a residue stated as a CONDITION with no count — a count is a completeness claim measurement cannot support"
    - "a claim that NAMES the rule that acts, rather than one asserting a layer governs; prefer a claim that stays true to one that must be maintained"
    - "an honesty repair ordered AFTER the severable rule, so it RECORDS an observed outcome rather than predicting one"
key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/envelope/ledger.rs
    - src/envelope/cred.rs
    - tests/envelope_interior_path.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The interior-path boundary is `/`-anchored substrings of a LITERAL word — not `=`, not a character list, not an option-spelling list"
  - "Containment is of the NORMALISED STRING, not of the index, so no refusal that existed can be lost"
  - "The residue is a CONDITION with NO count, handed to no pin, schedule or version witness"
  - "The `credential.helper` clause is SECTION `credential` + FINAL COMPONENT `helper`, never a substring test"
  - "The ledger KIND check goes inside the existing `Ok` arm; the link-following stat is kept"
  - "cred.rs names the rule that acts rather than claiming a layer governs"
metrics:
  duration: "one session"
  completed: 2026-09-04
actuals:
  tokens: 41000
  tasks: 4
  commits: 4
status: complete
---

# Phase 19 Plan 31: The path inside the word, and the two claims measurement denied — Summary

**All nine of `19-30`'s RED rows are green, the gate is 1871 with ZERO failures,
and both false claims the code made about this cell are corrected.**

## FIRST: `/gsd-secure-phase 19` is NOT cleared, and nothing here is a closure

**`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112` and `T-19-116` all remain OPEN at
`high`.** `T-19-113` and `T-19-115` remain open; **`T-19-115` gets NO rule and its
condition is RESTATED, not closed.**

**Whether `T-19-119`, `T-19-120` and `T-19-121` close is audit 12's judgement
rather than this plan's claim. This plan says NARROWED-or-CORRECTED and states
what remains open.**

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" is written anywhere.

## `19-30`'s RED list was confirmed STILL RED before any production line moved

Driven against the unmodified tree at `dd17bfb`, `--no-fail-fast`, verbatim:

```text
tests/envelope_interior_path.rs
test result: FAILED. 28 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.02s
failures:
    after_19_31_a_colon_attachment_inside_an_assignment_prefix_is_reached_too
    after_19_31_a_ledger_that_is_not_a_regular_file_is_refused_rather_than_read
    after_19_31_an_assignment_prefix_naming_a_protected_value_is_refused_and_the_unprotected_twin_is_not
    after_19_31_an_attachment_that_is_not_an_equals_sign_is_reached_the_same_way
    after_19_31_an_equals_attached_carrier_path_is_refused_over_both_protected_paths
    after_19_31_ordering_pin_a_is_the_three_way_git_dir_pin_with_the_one_character_restored
    after_19_31_ordering_pin_b_across_segments_the_first_refusal_still_wins
    after_19_31_policy_rs_5743s_own_dd_example_is_refused_in_the_direction_it_names

tests/envelope_wrapper_class.rs
test result: FAILED. 54 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
failures:
    a_command_carrying_a_carrier_path_inside_a_word_is_refused_after_19_31
```

**Exactly `19-30-SUMMARY.md`'s recorded set — nine rows, no more and no fewer.
Not one was already green.** Their transitions:

| row | before | after | commit |
|---|---|---|---|
| `…an_equals_attached_carrier_path…` | RED | **GREEN** | `f3406d5` |
| `…an_attachment_that_is_not_an_equals_sign…` | RED | **GREEN** | `f3406d5` |
| `…a_colon_attachment_inside_an_assignment_prefix…` | RED | **GREEN** | `f3406d5` |
| `…an_assignment_prefix_naming_a_protected_value…` | RED | **GREEN** | `f3406d5` |
| `…policy_rs_5743s_own_dd_example…` | RED | **GREEN** | `f3406d5` |
| `…ordering_pin_a…` | RED | **GREEN** | `f3406d5` |
| `…ordering_pin_b…` | RED | **GREEN** | `f3406d5` |
| `a_command_carrying_a_carrier_path_inside_a_word…` | RED | **GREEN** | `f3406d5` |
| `…a_ledger_that_is_not_a_regular_file…` | RED | **GREEN** | `72f6aaa` |

## `19-30`'s PERMITTED list, re-checked BEFORE and AFTER — the zero-move diff

```text
the_cost_rows_carry_an_equals_and_a_slash_and_stay_permitted_before_and_after   ok -> ok
the_near_miss_and_exact_path_controls_stay_permitted_before_and_after           ok -> ok
the_verdict_preserving_spellings_from_rounds_10_and_11_stay_permitted           ok -> ok
a_regular_ledger_just_under_the_bound_still_permits_and_still_counts…           ok -> ok
a_fresh_envelope_root_with_no_ledger_at_all_still_permits_before_and_after      ok -> ok
the_ledger_kind_reachability_table_is_measured_with_its_four_silences_beside_it ok -> ok
containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte         ok -> ok
every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule  ok -> ok
the_verdict_preserving_control_carrier_alphabets_stay_permitted…  (wrapper)     ok -> ok
wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic        ok -> ok
```

**ZERO moves.** Had a row moved from permitted to refused it would have been
reported as a finding about the RULE, never edited.

## The four commits, in order

| SHA | what | `src/` files |
|---|---|---|
| `f3406d5` | the candidate scan, the restated condition, `policy.rs:5743`, the `:9719` comment | `policy.rs` |
| `835d6c4` | **SEVERABLE:** the `credential.helper` clause | `policy.rs` |
| `72f6aaa` | **NON-SEVERABLE:** the ledger KIND check **and** the `cred.rs` claim repair | `ledger.rs`, `cred.rs` |
| `604ad2e` | the record | — |

**The claim correction and the severable rule are in DIFFERENT COMMITS**, and the
correction runs AFTER the rule so that it RECORDS an observed outcome rather than
predicting one. Under severance the plan would have had three commits and the
repair would still have landed.

`git diff --name-only dd17bfb..HEAD -- src/` lists exactly `policy.rs`,
`ledger.rs` and `cred.rs`. `git diff --numstat dd17bfb..HEAD -- tests/` shows
**164 additions and ZERO deletions**. Neither `Cargo.toml` nor `Cargo.lock`
appears.

## The design, with the boundary stated exactly

**Every `/`-ANCHORED SUBSTRING of a LITERAL word**, each fed through the
**EXISTING** `lexical_absolute_components` and the **EXISTING** `word_is_within` /
`word_is_exactly`. `lexical_absolute_components` is unchanged in behaviour and in
signature text.

**CONTAINMENT, STATED PRECISELY RATHER THAN AS THE `i == 0` APPROXIMATION.** The
normaliser strips leading `./`s **before** it tests `starts_with('/')`, so the
shorthand is false — `.//abs/p` is accepted today at no `/`-index of its own. The
exact statement, written into the helper's doc and into the record, and asserted
mechanically by `containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte`:
**for every word today's rule answers `Some(v)` for, the string it actually
NORMALISES — the word with its leading `./`s stripped — begins with `/`, is
therefore itself one of the candidates, and re-normalises to exactly `v`.**
Containment is of the NORMALISED STRING, not of the index; the `./` strip can only
ADD answers. **No refusal that existed before this round can have been lost.**

**It is not an attachment-character list, and that was forced rather than
preferred.** `dd`'s `of=` is an OPERAND grammar; `tar -C/p` attaches with nothing;
`rsync host:/p` with a `:`; `--opt=a=/p` after a second `=`. A character list is a
program-grammar enumeration — D-08's defect one level over — and
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` is
green.

**NO SECOND READING SITE WAS NEEDED AND NONE WAS WRITTEN.**
`protected_carrier_named` is still raised ONCE, at the top of `classify_segments`'
existing per-segment loop. **`hooks.rs` was not opened.** Ordering pin B's control
row — `git push --force … && dd if=… of=<ENV>/alpha/x` answering
`force_push_blocked` — is the mechanical proof, and it is green.

**ZERO DIFF LINES, read out of the diff:** `SEPARATORS` (byte-identical at
`policy.rs:2297`, still ONE commit in the phase, `84a9b05`), `is_separator`,
`tokenize`, `skip_redirection_target`, `redirection_operator_len`,
`split_segments_with_heads`, `Token`, `Segment`, `segment.tokens`,
`resolve_program`, `resolve_program_with_head`, `first_unreadable_decision_word`,
`config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api`,
`forbidden_repo_path`, `forbidden_repo_prefixes`, `POLICY_MIN_PRODUCTION_BYTES`,
`HOOKS_MIN_PRODUCTION_BYTES`. `is_separator(">")` is still `false`.

*(Two apparent `git diff` hits were checked by reading: `SEPARATORS` appears only
inside a moved doc sentence, and `forbidden_repo_path` only as a hunk-header
context label.)*

## The COST, disclosed from both sides

Every shape audit 11 named, exit 0 **before AND after** through the guard, **and
now at the unit level too with its candidate component lists asserted**:

```text
--author=A <a@b.c>                 no `/` in the word              -> NO CANDIDATE AT ALL
--format=%H                        no `/`                          -> NO CANDIDATE AT ALL
sed s/x/y/                         [x, y], [y], []                 -> shorter than any envelope dir
https://github.com/o/r             [github.com,o,r] x2, [o,r], [r] -> wrong FIRST component
HEAD:refs/heads/gsd-auto/alpha/w   [heads,gsd-auto,alpha,w], …     -> wrong FIRST component
git --git-dir=/tmp/g status        [tmp, g]                        -> not under <root>/<alias>
GSD_MM_ENVELOPE_ROOT=/tmp/fresh …  [tmp, fresh]                    -> not protected: PERMITTED
rm -f /tmp/pr-ledger.ndjson        [pr-ledger.ndjson]              -> shorter than the directory
rg pr-ledger.ndjson src/           `src/` -> the EMPTY component list
cp /bin/true <BINPAR>/some-other-file   never EQUALS the binary    -> the exact-path control holds
```

The mechanical reason is `word_is_within`'s `dir.is_empty() || word.len() < dir.len()`
early return over COMPONENT VECTORS, now applied PER CANDIDATE. **The cost is
bounded by the PATH SET, not by the split.** `git --git-dir=/tmp/g status` is
still permitted in all five of its sites.

**THE NEW OVER-REFUSAL SURFACE, written into the predicate's own cost section in
its GENERAL form rather than narrated by its easiest instance:**

> **Any word whose text contains THIS RUN'S OWN envelope directory or THIS RUN'S
> OWN binary path as a `/`-anchored substring is now refused — EVEN WHERE THE
> PROGRAM WOULD NOT HAVE USED THAT SUBSTRING AS A PATH.** As a value, a pattern, a
> commit message, a URL fragment, or a relative path that merely happens to
> contain it.

It fails CLOSED, it is bounded to this run's own two paths, and it is the same
family as the already-disclosed refusal of `cat <ledger>`. `19-30`'s measured
instances sit UNDER it: a RELATIVE word that textually contains the carrier path
(now pinned at the unit level beside the `./alpha/…` row it corrects), and a URL
whose `#`-fragment contains it — **whose `?`-spelling does NOT, because `?` clears
`Token.literal` and the scan never runs on the word.** That difference is why the
surface is described over what the predicate READS rather than over what a URL
looks like.

## The RESTATED RESIDUE CONDITION, quoted verbatim from the predicate's doc

> **This predicate reads every `/`-ANCHORED SUBSTRING of every LITERAL word, in
> either word class, against both paths. It is silent about a word the SHELL MAY
> REWRITE, because the guard cannot know its final text; about a word whose TEXT
> CARRIES NO ABSOLUTE PATH ANYWHERE, because there is nothing in it to normalise;
> and about a word that reaches a protected path ONLY THROUGH A LINK, because no
> lexical reading of the text names it.**

**The one-character correction:** *"a word that IS NOT ABSOLUTE"* → *"a word whose
TEXT carries no absolute path anywhere in it."* **NO COUNT is written.** The doc
previously said *"SEVEN spellings are MEASURED"* and before that *"four"*; each
number was correct only until the next round found a spelling it had not, which is
`T-19-107`'s shape in a shorter sentence. **Handed to NO pin, NO schedule and NO
version witness** — every clause is reachable by the driven agent TODAY.

## What closing `T-19-119` DOES and DOES NOT do for `T-19-116`

**Does:** every option-attached spelling of the binary is refused, **including
`dd if=/bin/true of=<BINARY>`, the spelling that moved a bare remote's `main`
`835b6be` → `40ad7f7`.**

**Does not** — all four measured at exit 0:

```text
exit=0  cp /bin/true $(command -v gsd-meta-manager)          <- expansion-borne
exit=0  cp /bin/true ~/.cargo/bin/gsd-meta-manager           <- tilde
exit=0  cd <binary-parent> && cp /bin/true gsd-meta-manager  <- relative
        a PATH symlink whose target current_exe() reports    <- link
```

**None of the four is an option attachment. `T-19-116` stays OPEN at `high`.**

## The slice placement and the fourth positive control

`slash_anchored_candidates` sits **immediately after**
`fn lexical_absolute_components(word: &str)` — the literal slice anchor of
`the_carrier_predicate_asks_the_filesystem_nothing_and_its_own_source_says_so` —
so it falls INSIDE that region. **A FOURTH positive control naming it was added**,
so a future slice that missed it fails loudly rather than certifying the head of
the region. The eight forbidden filesystem and process APIs still appear nowhere
in the sliced code, and the anchor's signature text is unchanged.

## The stale-comment correction at `policy.rs:9717-9721`

`word_is_within("./alpha/pr-ledger.ndjson")` still answers `false` — **the
assertion did not move.** Its stated reason did: *"stripping the leading `./` must
not turn a relative word into an absolute one"* was true of the code that read only
the whole word and is false of the code now, because that word DOES yield the
absolute candidates `/alpha/pr-ledger.ndjson` and `/pr-ledger.ndjson`. It answers
`false` for a DIFFERENT reason — two components against three. The comment carries
that WR-02 correction, and **a NEW row was added beside it**:
`./tmp/envroot/alpha/pr-ledger.ndjson` answering `true`, with its one-character
control `./tmp/envrooz/…` answering `false`, pinning the disclosed over-refusal at
its measured verdict.

## The ledger KIND check

Inside `record_and_check_in`'s **existing `Ok` arm**, before the size comparison,
in **the same `stat`** — one syscall already on this path, no second probe, no
TOCTOU window widened.

```text
FIFO ledger (stat size 0)     before: exit 124 after 20.02 s   after: exit 2 after 21 ms
8 366 000-byte regular ledger before: exit 0, 89000 -> 89001    after: exit 0, 89000 -> 89001
FRESH root, no ledger at all  before: exit 0, 1 ledger line     after: exit 0, 1 ledger line
```

**The `Err` arm keeps its behaviour**, so a fresh envelope root's first forge call
still falls through. **The link-following stat was kept and the non-following
variant was not substituted**, so a symlinked-to-regular ledger stays permitted and
counted; that variant's literal name is deliberately absent from `ledger.rs`,
because the verify step greps the file for it and writing it to forbid it would
make the gate pass vacuously. **`MAX_LEDGER_BYTES`' VALUE, derivation and
just-under/just-over discrimination are untouched** — a bound over the wrong
property is not corrected by moving the number. The refusal is
`ParkReason::EnvelopeAssertionFailed`, never `PrCapExceeded` (D-24), names the file
and the kind found (AR-19-11), and quotes no command back (SAFE-04).

**`T-19-120`'s BEHAVIOURAL HALF stays UNMEASURED and is claimed in NEITHER
direction.**

## Both claim corrections, quoted before and after

Each was required **regardless of whether any rule landed**.

### `policy.rs:5743`

**Before:** *"The guard cannot tell a read from a write without knowing every
program's grammar — is `dd if=X of=Y` a read of `X` or a write of `Y`? is `tee F`
a read?"*

**Measured FALSE:** `dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen` was **exit
0** — the guard refused it NEITHER way, so the example illustrated nothing.

**After:** the sentence is kept and a WR-02 note added beside it stating that it
was right-SOUNDING because the ambiguity is real, FALSE about this code because the
path sat after an `=` inside a word, and TRUE now because the same word's
`/`-anchored candidates are read so both the `if=` and the `of=` spelling reach the
path set. `tee F`, beside it, was refused throughout. **The severance fallback —
replacing the example with `tee F` — was not needed.**

### `cred.rs:420-425`

**Before:** *"the bound is stated rather than assumed: that spelling is
ARGV-VISIBLE and is already governed by `scan_leading`'s leading-option region and
layer 2's whole grammar."*

**Argv-VISIBLE it was; GOVERNED it was not.** `scan_leading` parsed the word and no
rule acted on it — the by-name deny covered `core.hooksPath`, round 8's clause
covered `include.path`, and nothing covered `credential.helper`, measured returning
the ambient secret on ONE permitted line.

**After:** the bullet says the spelling is argv-visible, then **NAMES the rule that
acts on it** (`policy::config_key_names_the_credential_helper`) and states at the
same weight what that rule does not reach. **Recorded in place: this is the SIXTH
instance in the phase of a residue whose stated bound was a layer that did not
enforce it** — `T-19-84`, `T-19-107`, `T-19-109`, `T-19-115` are the others — and
the repaired text states the discipline explicitly: **prefer a claim that stays
true to one that must be maintained.** A sentence asserting another layer governs
something must be kept true by every future editor of that layer; one naming the
rule that acts, or plainly naming its absence, is checkable at a glance. The text
says that if the named rule is ever removed, the honest edit is to say the residue
is UNBOUNDED — not to reach for another layer.

**WHICH BRANCH WAS OBSERVED, AND HOW IT WAS CHECKED.** Task 2 ran first and this
repair read the tree rather than predicting: `git log --oneline` showed Task 2's
commit `835d6c4`; `grep` found `config_key_names_the_credential_helper` defined at
`policy.rs:1500` and raised at `:596` inside `scan_leading`. **Observed: the clause
LANDED**, and the repair was written to that observation.

## The severable clause LANDED — Task 2 was NOT severed

`config_key_names_the_credential_helper` is a predicate over **SECTION `credential`
and FINAL COMPONENT `helper`**, folded case-insensitively the way git folds them
and never reading the subsection. **The shape is DERIVED from `19-30`'s measured
key-shape space**, not chosen: git folds a section and a final name and keeps a
subsection case-sensitive, so it reaches the URL-SCOPED `credential.<url>.helper` —
whose subsection is any URL, an open family no enumeration could close — and none
of the near misses. Raised beside the `core.hooksPath` deny in the region
`scan_leading` already parses, at `ParkReason::EnvelopeAssertionFailed`. **No new
park reason, no new reading site, no change to the walk, to
`config_key_operand_index` or to `subcommand_word_indices`, and `hooks.rs` was not
opened.** **It is NOT a substring or `contains` test** — `contains` refuses
`credential.helperx`, a key real git IGNORES.

```text
                                                guard before   guard after
-c credential.helper=store                      exit 0         exit 2  envelope_assertion_failed
-c CREDENTIAL.HELPER=store                      exit 0         exit 2
-c Credential.Helper=store                      exit 0         exit 2
-c credential.https://github.com.helper=store   exit 0         exit 2   <- URL-SCOPED
--config-env=credential.helper=EVILVAR          exit 0         exit 2   <- see below
-c credentialx.helper=store                     exit 0         exit 0   <- CONTROL
-c notcredential.helper=store                   exit 0         exit 0   <- CONTROL
-c credential.helperx=store                     exit 0         exit 0   <- CONTROL
-c credential.helper.x=store                    exit 0         exit 0   <- CONTROL
-c credential=store                             exit 0         exit 0   <- CONTROL
```

**Every verdict was driven as a `record_only` print and converted to an assertion
only after it was read.** The five near-miss controls are what make this a by-name
clause, and they are PERMITTED rather than refused at the same identifier —
`19-27`'s measured failure mode avoided.

**What it does NOT reach, at the same weight:** `GIT_CONFIG_PARAMETERS` (an
environment variable, not argv — refused today by a **DIFFERENT** mechanism, the
env-key deny at `hook_bypass_blocked`; `T-19-104` stays registered, and the row is
RECORDED in neither direction); the `git config` WRITING form (a file write, the
family `cred.rs`'s injected empty pair covers instead, recorded not asserted); and
any spelling that names no leading option.

**Over-refusal disclosed:** reading the KEY half only means `-c credential.helper=`
with an EMPTY value — a reset — is refused too. It costs nothing reachable, because
the envelope already injects exactly that empty pair.

**`envelope_config_resolution` is 30/0 and NOT ONE of its thirty verdicts moved**,
confirmed before the commit and again at the gate.

## Mechanism pins and byte floors, re-measured

`SEPARATORS` byte-identical (ONE commit in the phase, `84a9b05`);
`is_separator(">")` `false`; `segment.tokens` byte-identical with the SEGMENT-COUNT
pins green in both variants; round 5's literalness bit non-vacuous; round 6's
deletion model unchanged with its over-deletion control at exit 0; round 7's four
callee-grammar directions; round 8's confinement clause; round 9's re-parse clause
with `aliasx.`/`notalias.` at exit 0 and both `T-19-86` `!`-bodied rows at exit 0;
round 10's clause; round 11's redirection and exact-path clauses with both
near-miss controls permitted.
**`every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule`
— nine unit pins, twenty fenced corpus words, eight positive controls — is GREEN**,
the mechanical proof the fenced-file enumeration is zero.
`POLICY_MIN_PRODUCTION_BYTES`, `HOOKS_MIN_PRODUCTION_BYTES`, the proportional
floor, the deep anchor and the one-`#[cfg(test)]`-sentinel counts are untouched.

## `hooks.rs`, `mod.rs`, `advisory.rs`, `scan.rs` and `config.rs` were NOT opened

`git diff --numstat dd17bfb..HEAD` over those five files is **empty** — half what
`19-29` opened. **`SECTION_ENVELOPE` shows zero diff lines and its headroom is
unspent**: `19-30` re-measured 211 whitespace tokens of an UNRAISED 215 cap, widest
line 74 of 80, and *"As started"* means `T-19-121` does not falsify the first
`Guaranteed` clause.

## THE GATE — the arithmetic, stated and CHECKED

`rtk proxy cargo test --no-fail-fast`, counted with `rtk proxy grep` over a
redirected log (D-34). `cargo build` and `cargo clippy -- -D warnings` both exit 0;
`cargo clippy --tests` was NOT the gate.

| | `19-30` | `19-31` |
|---|---|---|
| `passed + failed` | **1859** | **1871** |
| failures | 9, by design | **0** |
| result lines | 47 | 47 |
| ignored | 13 | 13 |
| `envelope_*` binaries | 18 | **18** |

**1871 − 1859 = 12, and 12 is exactly the number of new `#[test]` fns**, counted
from `git show` over the three code commits: **7** in `policy.rs`'s own `mod tests`
and **5** in `tests/envelope_interior_path.rs`. **A red test RAN, so red→green
leaves the total unchanged and every increase came ONLY from new tests.**

### The per-binary counts — ALL EIGHTEEN `envelope_*` binaries RAN

```text
envelope_advisory.rs          ok  10 passed  0 failed
envelope_argv_deletion.rs     ok  20 passed  0 failed
envelope_callee_grammar.rs    ok  19 passed  0 failed
envelope_carrier_reach.rs     ok  39 passed  0 failed
envelope_command_position.rs  ok  18 passed  0 failed
envelope_config_resolution.rs ok  30 passed  0 failed   <- the 30/0 pin, unmoved
envelope_control_carrier.rs   ok  37 passed  0 failed
envelope_credential.rs        ok   6 passed  0 failed
envelope_expansion_slots.rs   ok  32 passed  0 failed
envelope_hook_refusals.rs     ok   7 passed  0 failed
envelope_interior_path.rs     ok  41 passed  0 failed   <- was 28 passed / 8 failed
envelope_literal_decision.rs  ok  43 passed  0 failed
envelope_pr_cap.rs            ok  11 passed  0 failed
envelope_reparsed_value.rs    ok  34 passed  0 failed
envelope_tracer.rs            ok   6 passed  0 failed
envelope_wiring.rs            ok  14 passed  0 failed
envelope_wrapper_bypass.rs    ok  13 passed  0 failed
envelope_wrapper_class.rs     ok  55 passed  0 failed   <- was 54 passed / 1 failed
                                 435 tests over 18 binaries
```

**NONE of the four documented flakes fired** — not the two `driver_reattach`
failures, not `envelope_tracer`'s `ExecutableFileBusy` stub-write race, and not the
ETXTBSY race over the binary. **Absence is not evidence any of them is fixed**, and
nothing here was done to them. **The ETXTBSY race is `C-10`'s own seam and this
round changes what the guard does about that seam; its relation to this round is
claimed in NEITHER direction.**

## Deviations from plan

### 1. [Finding — measurement correcting a planning-time expectation] `--config-env` IS reached by the `credential.helper` clause

- **Found during:** Task 2, on the `record_only` probe run before any pin was
  written.
- **Issue:** the plan and `19-30` both record `--config-env=credential.helper=<VAR>`
  as a shape a by-name clause would **NOT** reach, and the plan mandates stating it
  as unreached "at the same weight". **It IS reached**, in both `--config-env`
  grammars, because `leading_git_option` yields an assignment for it and the clause
  reads the KEY half, which `--config-env` spells on argv exactly as `-c` does.
- **Disposition:** **a GAIN, recorded as a correction rather than aimed at.** Real
  git was measured resolving the helper from it (`19-30`), so it is a genuine reach
  closed rather than an over-refusal. The doc and the record state it as reached,
  and the "does not reach" list carries only what is actually unreached.
- **Commit:** `835d6c4`

### 2. [Finding — the plan's own gate would have failed vacuously] the forbidden API's name could not be written in `ledger.rs`

- **Found during:** Task 3's verify.
- **Issue:** the kind check's doc must say that the link-NON-following stat must
  not be substituted. Writing that API's literal name in `ledger.rs` makes the
  plan's own `grep -c '<name>' src/envelope/ledger.rs -eq 0` gate fail.
- **Disposition:** **not a defect — the plan anticipated it explicitly** and said
  the gate reads a different file than the plan does. The comment names the API by
  description rather than by spelling, and says so in place, so the gate stays a
  real check on the mistake it exists to catch rather than passing vacuously.
- **Commit:** `72f6aaa`

### 3. [Honesty] `19-30`'s zero-deletion conflict did NOT recur

- `19-30` reported that the plan's zero-deletions check over `tests/` cannot
  coexist with a mandate to raise floors. **This round raises no test-side floor**,
  and `git diff --numstat dd17bfb..HEAD -- tests/` is **164 additions, 0
  deletions**. The mechanical proxy and the semantic rule agree here. Reported
  rather than assumed to generalise.

### 4. [Rule 2 — required for actionability] `envelope_carrier_refusal`'s message was extended

- **Found during:** Task 1, step 3 (read and change only if measurement requires).
- **Issue:** both messages said the path is read *"whether it stands as an operand
  or after a redirection operator"*. After the widening that sentence
  under-describes the reach — a user running `tar -C<dir> …` would read "operand"
  and conclude the guard was wrong, which is how a control gets switched off
  (AR-19-11).
- **Fix:** extended minimally to *"whether it stands alone as a word, is carried
  INSIDE a longer word — after an option's `=`, attached to a short option, or
  after any other character — or follows a redirection operator"*. No command is
  quoted back (SAFE-04); the recovery step is unchanged. No test asserts the
  message text, verified by grep before the edit.
- **Commit:** `f3406d5`

## Self-Check: PASSED

```
FOUND: src/envelope/policy.rs
FOUND: src/envelope/ledger.rs
FOUND: src/envelope/cred.rs
FOUND: tests/envelope_interior_path.rs
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
FOUND: f3406d5  feat(19-31): the path INSIDE the word
FOUND: 835d6c4  feat(19-31): the credential.helper clause
FOUND: 72f6aaa  fix(19-31): the ledger's KIND inside the existing stat
FOUND: 604ad2e  docs(19-31): the round-13 record
git diff --name-only dd17bfb..HEAD -- src/   -> policy.rs, ledger.rs, cred.rs ONLY
git diff --numstat dd17bfb..HEAD -- hooks.rs mod.rs advisory.rs scan.rs config.rs -> EMPTY
git diff --numstat dd17bfb..HEAD -- tests/   -> 164 additions, 0 deletions
grep -c 'symlink_metadata' src/envelope/ledger.rs             -> 0
grep -c 'MAX_LEDGER_BYTES: u64 = 8 * 1024 * 1024' ledger.rs   -> 1
grep -cE '^\| AR-19-13 \|' 19-SECURITY.md                     -> 0
Cargo.toml / Cargo.lock in the diff                           -> ABSENT
passed + failed = 1871; failed = 0; envelope_* binaries = 18
new #[test] fns from git show = 12; 1871 - 1859 = 12          -> ARITHMETIC HOLDS
all 9 carry-forward RED names asserted GREEN by name          -> PASS
```

## Known Stubs

None. No stub, placeholder or hardcoded empty value was written; every row is
either driven against the built guard or asserted over the real production
functions.

## What remains uncovered

**`T-19-86` FIRST — OPEN at `high` by explicit user scoping decision.** Not fixed,
not narrowed, not re-scoped, not re-classified; its registered rows stay at exit 0
and its pins green and UNMODIFIED, and **`T-19-111` is kept OUT of it**.

Then, open at `high`: **`T-19-91`** (three arms unweakened); **`T-19-111`** (no
rule, `19-27`'s five-site attribution correction unchanged); **`T-19-112`**
(narrowed further, not closed); **`T-19-116`** (**NARROWED FURTHER, NOT CLOSED**,
four residues at exit 0, all named above).

Then: **`T-19-113`**; **`T-19-115`** (no rule; the condition is RESTATED, not
closed, and no acceptance is made); **`T-19-119`**, **`T-19-120`** and
**`T-19-121`** (**narrowed or corrected — none closed by this plan's claim; that
is audit 12's judgement**); **`T-19-96`**, **`T-19-110`**, **`T-19-74`** (core rows
frozen); **`T-19-84`**, **`T-19-85`** and **`T-19-61` … `T-19-73`** (open and
unaccepted by explicit user decision; `scan.rs` and `config.rs` were not opened).

**`C-08`'s and `T-19-120`'s behavioural halves stay UNMEASURED and are claimed in
neither direction. `C-11` … `C-15` keep control (e) with no rule and no `pr_cap_*`
clamp. The `glab --host` forge cell is unfixed, `FORGE_VALUE_OPTS` keeps
`--hostname`, and `glab` is confirmed NOT INSTALLED.** `T-19-104` stays registered
at exit 2 `hook_bypass_blocked` — a different mechanism from this round's clause.

**`T-19-17r` is OUTSTANDING for the FIFTEENTH time.** This plan did NOT accept it,
added no Accepted-Risks-Log row, created no `AR-19-13`, and applied the word
"accepted" to it nowhere. `AR-19-04` and `AR-19-05` were not un-accepted, re-rated
or renumbered, and **`T-19-23` was not marked closed**.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**
